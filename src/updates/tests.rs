use super::*;
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::Command,
    sync::atomic::AtomicU64,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rstop update {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn server(status: u16, location: Option<&str>, body: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/fixture", listener.local_addr().unwrap());
    let location = location.map_or(String::new(), |url| format!("Location: {url}\r\n"));
    let response = format!(
        "HTTP/1.1 {status} Fixture\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        started.elapsed() < Duration::from_secs(15),
                        "No fixture request"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0 && request.len() < 16 * 1024);
            request.extend_from_slice(&buffer[..count]);
        }
        let request = String::from_utf8(request).unwrap();
        assert!(request.starts_with("GET /fixture HTTP/1.1\r\n"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        stream.write_all(response.as_bytes()).unwrap();
    });
    (url, handle)
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

#[test]
fn public_redirect_resolves_without_following_the_page_or_querying_the_api() {
    let location = format!("{RELEASES}v99.0.0");
    let (url, handle) = server(302, Some(&location), "");
    let release = latest_with(&client(), &url).unwrap();
    handle.join().unwrap();
    assert!(release.newer_than(env!("CARGO_PKG_VERSION")));

    // Any attempted API lookup fails immediately, so this also proves that the
    // resolved metadata survives AxoUpdater's normal version/receipt checks.
    let mut updater = axoupdater::AxoUpdater::new_for("rstop");
    updater.set_install_dir(
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_str()
            .unwrap(),
    );
    updater
        .set_current_version(env!("CARGO_PKG_VERSION").parse().unwrap())
        .unwrap();
    updater.set_client(
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:1").unwrap())
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap(),
    );
    let resolved = resolved_release(&release).unwrap();
    for asset in &resolved.assets {
        assert_eq!(
            asset.browser_download_url,
            format!("{DOWNLOADS}v99.0.0/{}", asset.name)
        );
    }
    updater.configure_release(resolved);
    assert!(updater.is_update_needed_sync().unwrap());
}

#[test]
fn redirects_reject_other_repositories_prereleases_and_untrusted_urls() {
    for url in [
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0",
        "https://github.com/fan-ziqi/rstop/releases/tag/99.0.0",
    ] {
        assert!(release_from_url(url).is_ok());
    }
    for url in [
        "https://github.com/fan-ziqi/rstop/releases/latest",
        "https://github.com/fan-ziqi/other/releases/tag/v99.0.0",
        "https://github.com/other/rstop/releases/tag/v99.0.0",
        "http://github.com/fan-ziqi/rstop/releases/tag/v99.0.0",
        "https://github.com.evil.test/fan-ziqi/rstop/releases/tag/v99.0.0",
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0-rc.1",
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0?redirect=evil",
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0#fragment",
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0/installer.sh",
        "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0%2Finstaller.sh",
    ] {
        assert!(release_from_url(url).is_err(), "{url}");
    }
}

#[test]
fn failed_or_incomplete_checks_never_report_up_to_date() {
    for (status, location) in [
        (403, None),
        (429, None),
        (404, None),
        (500, None),
        (200, None),
        (302, None),
        (302, Some("https://example.com/v99.0.0")),
        (
            304,
            Some("https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0"),
        ),
    ] {
        let (url, handle) = server(status, location, "rate limit exceeded");
        assert!(latest_with(&client(), &url).is_err(), "{status}");
        handle.join().unwrap();
    }
}

#[test]
fn resolved_installer_replaces_the_owned_copy_and_restores_it_on_failure() {
    for (status, install_succeeds) in [(200, true), (200, false), (403, false), (500, false)] {
        let fixture = Fixture::new();
        let managed = fixture.0.join("managed");
        fs::create_dir(&managed).unwrap();
        let binary = managed.join(if cfg!(windows) { "rstop.exe" } else { "rstop" });
        fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
        let receipt = fixture.0.join("rstop-receipt.json");
        let original = serde_json::to_vec(&serde_json::json!({
            "install_prefix": managed, "binaries": [binary.file_name().unwrap().to_str().unwrap()],
            "source": {"app_name": "rstop", "owner": "fan-ziqi", "name": "rstop", "release_type": "github"},
            "version": env!("CARGO_PKG_VERSION"),
            "provider": {"source": "cargo-dist", "version": "0.33.1"}
        })).unwrap();
        fs::write(&receipt, &original).unwrap();
        let script = if status == 200 && !install_succeeds {
            if cfg!(windows) {
                "exit 7\n"
            } else {
                "#!/bin/sh\nexit 7\n"
            }
        } else if cfg!(windows) {
            "$ErrorActionPreference = 'Stop'\nSet-Content -LiteralPath (Join-Path $env:RSTOP_INSTALL_DIR 'rstop.exe') -Value 'owned replacement' -NoNewline\n"
        } else {
            "#!/bin/sh\nset -eu\nrm -f \"$RSTOP_INSTALL_DIR/rstop\"\nprintf 'owned replacement' > \"$RSTOP_INSTALL_DIR/rstop\"\n"
        };
        let (url, handle) = server(status, None, script);
        let output = Command::new(&binary)
            .args(["--exact", "updates::tests::installer_child", "--nocapture"])
            .env_remove("AXOUPDATER_CONFIG_WORKING_DIR")
            .env("AXOUPDATER_CONFIG_PATH", &fixture.0)
            .env("RSTOP_TEST_INSTALLER_URL", url)
            .env(
                "RSTOP_TEST_INSTALLER_SUCCESS",
                if install_succeeds { "1" } else { "0" },
            )
            .output()
            .unwrap();
        handle.join().unwrap();
        assert!(
            output.status.success(),
            "{status}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        if install_succeeds {
            assert_eq!(fs::read(&binary).unwrap(), b"owned replacement");
        } else {
            assert_eq!(
                fs::read(&binary).unwrap(),
                fs::read(std::env::current_exe().unwrap()).unwrap()
            );
        }
        assert_eq!(fs::read(&receipt).unwrap(), original);
    }
}

#[test]
fn installer_child() {
    let Ok(url) = std::env::var("RSTOP_TEST_INSTALLER_URL") else {
        return;
    };
    let release = release_from_url(&format!("{RELEASES}v99.0.0")).unwrap();
    let mut resolved = resolved_release(&release).unwrap();
    for asset in &mut resolved.assets {
        asset.browser_download_url = url.clone();
    }
    let mut updater = updater().unwrap();
    updater.set_client(
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
    );
    updater
        .disable_installer_output()
        .configure_release(resolved);
    let result = updater.run_sync();
    if std::env::var("RSTOP_TEST_INSTALLER_SUCCESS").unwrap() == "1" {
        assert_eq!(result.unwrap().unwrap().new_version.to_string(), "99.0.0");
    } else {
        assert!(result.is_err());
    }
}

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
    assert!(matches!(&notice.phase, Phase::CheckFailed(error) if error == "Network unavailable"));
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
