//! Quiet background checks and explicit, receipt-bound updates from GitHub Releases.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{sync::atomic::Ordering, sync::mpsc, thread, time::Duration};

const LATEST: &str = "https://github.com/fan-ziqi/rstop/releases/latest";
const RELEASES: &str = "https://github.com/fan-ziqi/rstop/releases/tag/";
const DOWNLOADS: &str = "https://github.com/fan-ziqi/rstop/releases/download/";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
}

impl Release {
    pub fn version(&self) -> &str {
        self.tag_name.strip_prefix('v').unwrap_or(&self.tag_name)
    }

    fn is_stable(&self) -> bool {
        !self.draft
            && !self.prerelease
            && semver::Version::parse(self.version()).is_ok_and(|version| version.pre.is_empty())
            && self.html_url == format!("{RELEASES}{}", self.tag_name)
    }

    pub fn newer_than(&self, current: &str) -> bool {
        let Ok(latest) = semver::Version::parse(self.version()) else {
            return false;
        };
        let Ok(current) = semver::Version::parse(current) else {
            return false;
        };
        self.is_stable() && latest > current
    }
}

#[derive(Debug)]
pub enum Event {
    Available(Release),
    Checked(std::result::Result<Release, String>),
    Installed(std::result::Result<String, String>),
}

#[derive(Clone, Debug)]
pub enum Phase {
    Checking,
    UpToDate,
    CheckFailed(String),
    Available,
    Installing,
    Installed(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub release: Release,
    pub phase: Phase,
    pub selected: usize,
}

impl Notice {
    pub fn checking() -> Self {
        Self {
            phase: Phase::Checking,
            ..Self::new(Release::default())
        }
    }

    pub fn finish_check(&mut self, result: std::result::Result<Release, String>) {
        self.selected = 0;
        self.phase = match result {
            Ok(release) => {
                if !release.is_stable() {
                    Phase::CheckFailed("GitHub returned an invalid stable release.".into())
                } else {
                    let newer = release.newer_than(env!("CARGO_PKG_VERSION"));
                    self.release = release;
                    if newer {
                        Phase::Available
                    } else {
                        Phase::UpToDate
                    }
                }
            }
            Err(error) => Phase::CheckFailed(error),
        };
    }

    pub fn new(release: Release) -> Self {
        Self {
            release,
            phase: Phase::Available,
            selected: 0,
        }
    }
}

pub fn check_once_background(sender: mpsc::Sender<Event>) {
    let spawn = thread::Builder::new()
        .name("rstop-manual-update-check".into())
        .spawn({
            let sender = sender.clone();
            move || {
                let result =
                    latest().map_err(|error| format!("Could not check for updates: {error:#}"));
                let _ = sender.send(Event::Checked(result));
            }
        });
    if let Err(error) = spawn {
        let _ = sender.send(Event::Checked(Err(error.to_string())));
    }
}

/// Check once at startup; current versions and network failures stay silent.
pub fn check_background(sender: mpsc::Sender<Event>) {
    let _ = thread::Builder::new()
        .name("rstop-update-check".into())
        .spawn(move || {
            if !crate::STOP.load(Ordering::Relaxed)
                && let Ok(release) = latest()
                && release.newer_than(env!("CARGO_PKG_VERSION"))
            {
                let _ = sender.send(Event::Available(release));
            }
        });
}

pub fn latest() -> Result<Release> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("rstop/", env!("CARGO_PKG_VERSION")))
        .build()?;
    latest_with(&client, LATEST)
}

fn latest_with(client: &reqwest::blocking::Client, url: &str) -> Result<Release> {
    // GitHub's documented /releases/latest link resolves to the stable tag.
    // Read its redirect header without fetching the page or spending REST API quota.
    let response = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("Could not reach GitHub Releases; retry or rerun the official installer")?;
    ensure!(
        matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308),
        "GitHub did not redirect to the latest rstop release"
    );
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .context("GitHub did not return a release location")?
        .to_str()?;
    release_from_url(location)
}

fn release_from_url(url: &str) -> Result<Release> {
    let release = Release {
        tag_name: url.strip_prefix(RELEASES).unwrap_or_default().into(),
        html_url: url.into(),
        ..Default::default()
    };
    ensure!(
        release.is_stable(),
        "GitHub did not return a valid stable rstop release"
    );
    Ok(release)
}

fn resolved_release(release: &Release) -> Result<axoupdater::Release> {
    ensure!(release.is_stable(), "Invalid update release");
    Ok(axoupdater::Release {
        tag_name: release.tag_name.clone(),
        version: release.version().parse()?,
        name: release.tag_name.clone(),
        url: release.html_url.clone(),
        prerelease: false,
        assets: ["rstop-installer.sh", "rstop-installer.ps1"]
            .into_iter()
            .map(|name| {
                let url = format!("{DOWNLOADS}{}/{name}", release.tag_name);
                axoupdater::Asset {
                    url: url.clone(),
                    browser_download_url: url,
                    name: name.into(),
                }
            })
            .collect(),
    })
}

/// Install the displayed version using the normal receipt and platform guards.
pub fn install_release(updater: &mut axoupdater::AxoUpdater, release: &Release) -> Result<String> {
    ensure!(
        release.newer_than(env!("CARGO_PKG_VERSION")),
        "Invalid update release"
    );
    updater.configure_release(resolved_release(release)?);
    let result = updater.run_sync()?.context("No update was installed")?;
    Ok(result.new_version.to_string())
}

/// All callers share the same executable/receipt guard and network deadlines.
pub fn updater() -> Result<axoupdater::AxoUpdater> {
    let mut updater = axoupdater::AxoUpdater::new_for("rstop");
    updater.load_receipt().context("No valid rstop install receipt. Install rstop with the official installer before using 'rstop update'")?;
    ensure!(
        updater.check_receipt_is_for_this_executable()?,
        "This executable is not the installer-managed copy of rstop. Update the installed copy or rerun the official installer"
    );
    updater.set_client(
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("rstop/", env!("CARGO_PKG_VERSION")))
            .build()?,
    );
    if let Ok(token) = std::env::var("RSTOP_GITHUB_TOKEN") {
        updater.set_github_token(&token);
    }
    Ok(updater)
}

pub fn install_background(release: Release, sender: mpsc::Sender<Event>) {
    let spawn = thread::Builder::new().name("rstop-upgrade".into()).spawn({
        let sender = sender.clone();
        move || {
            let result = (|| -> Result<String> {
                let mut updater = updater()?;
                updater.disable_installer_output();
                install_release(&mut updater, &release)
            })()
            .map_err(|error| format!("{error:#}"));
            let _ = sender.send(Event::Installed(result));
        }
    });
    if let Err(error) = spawn {
        let _ = sender.send(Event::Installed(Err(error.to_string())));
    }
}

#[cfg(test)]
mod tests;
