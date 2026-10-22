use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rstop-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rstop"));
        command.args(["--config", self.0.join("config.toml").to_str().unwrap()]);
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn informational_subcommands_ignore_invalid_monitor_configuration() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("config.toml"), "invalid = [").unwrap();
    for args in [
        &["completions", "bash"][..],
        &["man"],
        &["config", "path"],
        &["update", "--help"],
    ] {
        let output = fixture.command().args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn replay_exports_use_subcommands_and_preserve_the_input() {
    let fixture = Fixture::new();
    let input = fixture.0.join("session.jsonl");
    let csv = fixture.0.join("metrics.csv");
    let original = b"{\"timestamp\":42,\"hostname\":\"owned-fixture\"}\n";
    fs::write(&input, original).unwrap();
    let output = fixture
        .command()
        .arg("replay")
        .arg(&input)
        .args(["--samples", "1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot["timestamp"], 42);
    let output = fixture
        .command()
        .args(["export", "csv"])
        .arg(&csv)
        .arg("--input")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(csv)
            .unwrap()
            .starts_with("timestamp,path,value\n")
    );
    let output = fixture
        .command()
        .args(["export", "csv"])
        .arg(&input)
        .arg("--input")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(input).unwrap(), original);
}

#[test]
fn update_requires_a_receipt_for_the_running_executable_before_network_access() {
    let fixture = Fixture::new();
    let mut command = fixture.command();
    command
        .env_remove("AXOUPDATER_CONFIG_WORKING_DIR")
        .env("AXOUPDATER_CONFIG_PATH", &fixture.0)
        .env("HTTPS_PROXY", "http://127.0.0.1:1");
    let output = command.args(["update", "--check"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No valid rstop install receipt"));

    let other = fixture.0.join("other-copy");
    fs::create_dir_all(&other).unwrap();
    fs::write(fixture.0.join("rstop-receipt.json"), serde_json::to_vec(&serde_json::json!({
        "install_prefix": other, "binaries": ["rstop"],
        "source": {"app_name": "rstop", "owner": "fan-ziqi", "name": "rstop", "release_type": "github"},
        "version": env!("CARGO_PKG_VERSION"), "provider": {"source": "cargo-dist", "version": "0.33.1"}
    })).unwrap()).unwrap();
    let mut command = fixture.command();
    let output = command
        .env_remove("AXOUPDATER_CONFIG_WORKING_DIR")
        .env("AXOUPDATER_CONFIG_PATH", &fixture.0)
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .arg("update")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not the installer-managed copy"));
}

#[test]
fn offline_doctor_and_network_cli_validate_without_running_live_actions() {
    let fixture = Fixture::new();
    let input = fixture.0.join("diagnosis.jsonl");
    fs::write(&input,b"{\"timestamp\":42,\"hostname\":\"owned\",\"disks\":[{\"mount\":\"/owned\",\"total\":1000,\"used\":990,\"available\":10}]}\n").unwrap();
    let output = fixture
        .command()
        .args(["doctor", "--json", "--view", "storage", "--input"])
        .arg(&input)
        .env("PATH", &fixture.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["recorded"], true);
    assert_eq!(report["findings"][0]["severity"], "URGENT");
    for args in [
        vec!["network", "capture", "owned.pcap", "--seconds", "0"],
        vec!["--read-only", "network", "capture", "owned.pcap"],
        vec!["network", "check", "https://user:secret@example.com"],
    ] {
        let output = fixture
            .command()
            .args(args)
            .current_dir(&fixture.0)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!fixture.0.join("owned.pcap").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn configured_commands_do_not_run_in_read_only_or_no_config_exec_mode() {
    let fixture = Fixture::new();
    let marker = fixture.0.join("must-not-exist");
    fs::write(
        fixture.0.join("config.toml"),
        format!(
            r#"[[probes]]
name = "owned-exec-fixture"
command = ["touch", {:?}]
"#,
            marker.to_string_lossy()
        ),
    )
    .unwrap();
    for mode in ["--read-only", "--no-config-exec"] {
        let output = fixture
            .command()
            .args([mode, "--no-gpu", "--interval", "250", "snapshot"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!marker.exists());
    }
}

#[test]
fn saving_no_exec_preferences_preserves_rules_without_persisting_the_runtime_permission() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("config.toml"),
        r#"alert_commands = [["unused-not-run"]]
[[probes]]
name = "preserved-rule"
command = ["unused-not-run"]
"#,
    )
    .unwrap();
    let output = fixture
        .command()
        .args(["--no-config-exec", "config", "write"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(fixture.0.join("config.toml")).unwrap();
    let config: rstop::config::Config = toml::from_str(&text).unwrap();
    assert_eq!(config.probes.len(), 1);
    assert_eq!(config.alert_commands.len(), 1);
    assert!(config.config_exec);
    assert!(!text.contains("config_exec"));
}
