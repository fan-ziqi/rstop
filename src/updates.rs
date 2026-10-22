//! Quiet background checks and explicit, receipt-bound updates from GitHub Releases.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{io::Read, sync::atomic::Ordering, sync::mpsc, thread, time::Duration};

const API: &str = "https://api.github.com/repos/fan-ziqi/rstop/releases/latest";
const RELEASES: &str = "https://github.com/fan-ziqi/rstop/releases/tag/";
const CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);
const MAX_RESPONSE: u64 = 512 * 1024;

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

    pub fn newer_than(&self, current: &str) -> bool {
        let Ok(latest) = semver::Version::parse(self.version()) else {
            return false;
        };
        let Ok(current) = semver::Version::parse(current) else {
            return false;
        };
        !self.draft
            && !self.prerelease
            && latest.pre.is_empty()
            && latest > current
            && self.html_url == format!("{RELEASES}{}", self.tag_name)
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
                let eligible = !release.draft
                    && !release.prerelease
                    && semver::Version::parse(release.version())
                        .is_ok_and(|version| version.pre.is_empty())
                    && release.html_url == format!("{RELEASES}{}", release.tag_name);
                if !eligible {
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

pub fn check_background(sender: mpsc::Sender<Event>) -> Option<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("rstop-update-check".into())
        .spawn(move || {
            while !crate::STOP.load(Ordering::Relaxed) {
                if let Ok(release) = latest()
                    && release.newer_than(env!("CARGO_PKG_VERSION"))
                    && sender.send(Event::Available(release)).is_err()
                {
                    break;
                }
                thread::park_timeout(CHECK_INTERVAL);
            }
        })
        .ok()
}

pub fn latest() -> Result<Release> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("rstop/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let mut request = client
        .get(API)
        .header("Cache-Control", "no-cache")
        .header("Accept", "application/vnd.github+json");
    if let Ok(token) = std::env::var("RSTOP_GITHUB_TOKEN") {
        request = request.bearer_auth(token);
    }
    let response = request.send()?.error_for_status()?;
    Ok(serde_json::from_reader(response.take(MAX_RESPONSE))?)
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
                ensure!(
                    release.newer_than(env!("CARGO_PKG_VERSION")),
                    "Invalid update release"
                );
                let mut updater = updater()?;
                updater
                    .disable_installer_output()
                    .configure_version_specifier(axoupdater::UpdateRequest::SpecificTag(
                        release.tag_name,
                    ));
                let result = updater.run_sync()?.context("No update was installed")?;
                Ok(result.new_version.to_string())
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
mod tests {
    use super::*;

    #[test]
    fn only_newer_stable_releases_from_this_repository_are_eligible() {
        let mut release = Release {
            tag_name: "v0.1.10".into(),
            html_url: format!("{RELEASES}v0.1.10"),
            ..Release::default()
        };
        assert!(release.newer_than("0.1.9"));
        assert!(!release.newer_than("0.2.0"));
        assert!(!release.newer_than("0.1.10"));
        release.prerelease = true;
        assert!(!release.newer_than("0.1.9"));
        release.prerelease = false;
        release.html_url = "https://example.com".into();
        assert!(!release.newer_than("0.1.9"));
    }

    #[test]
    fn upgrade_is_the_default_choice() {
        assert_eq!(Notice::new(Release::default()).selected, 0);
    }

    #[test]
    fn manual_checks_report_available_current_invalid_and_failed_results() {
        let mut notice = Notice::checking();
        let release = |version: &str| Release {
            tag_name: format!("v{version}"),
            html_url: format!("{RELEASES}v{version}"),
            ..Default::default()
        };
        assert!(matches!(notice.phase, Phase::Checking));
        notice.finish_check(Ok(release("99.0.0")));
        assert!(matches!(notice.phase, Phase::Available));
        assert_eq!(notice.release.version(), "99.0.0");
        for version in [env!("CARGO_PKG_VERSION"), "0.0.1"] {
            notice.finish_check(Ok(release(version)));
            assert!(matches!(notice.phase, Phase::UpToDate));
        }
        notice.finish_check(Err("Network unavailable".into()));
        assert!(
            matches!(&notice.phase, Phase::CheckFailed(error) if error == "Network unavailable")
        );
        for invalid in [
            Release::default(),
            release("invalid"),
            Release {
                html_url: "https://example.com".into(),
                ..release("99.0.0")
            },
            Release {
                prerelease: true,
                ..release("99.0.0")
            },
        ] {
            notice.finish_check(Ok(invalid));
            assert!(matches!(notice.phase, Phase::CheckFailed(_)));
        }
    }
}
