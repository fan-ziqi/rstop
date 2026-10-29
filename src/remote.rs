//! Interactive monitoring of another host through the system OpenSSH client.
//!
//! The remote host runs its own rstop inside an SSH terminal, so every page and
//! action applies to that host. When rstop is missing there and the platforms
//! match, this build is copied to the remote user's cache directory first.
use crate::{Args, commands::Command};
use anyhow::{Context, Result};
use clap::Parser;
use rstop::STOP;
use std::{
    hash::Hasher,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::{self, Stdio},
    sync::atomic::Ordering,
};

/// Launch script status when no rstop executable exists on the remote host.
const MISSING: i32 = 97;
const INSTALLER: &str = "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fan-ziqi/rstop/releases/latest/download/rstop-installer.sh | sh";

/// Clap captures `DESTINATION [ARGS]` as an external subcommand. Parse the
/// command line again without the destination so options after it, and the
/// gpu/network views, keep their usual meaning.
pub fn split(external: &[String]) -> (String, Args) {
    let mut argv = std::env::args_os().collect::<Vec<_>>();
    // The external subcommand is the verbatim tail of the command line.
    argv.remove(argv.len() - external.len());
    (external[0].clone(), Args::parse_from(argv))
}

pub fn run(destination: &str, mut args: Args) -> Result<()> {
    let target = Target::parse(destination)?;
    let unsupported = match args.command.take() {
        None | Some(Command::Monitor) => false,
        Some(Command::Gpu { command: None }) => {
            args.view = Some("gpu".into());
            false
        }
        Some(Command::Network { command: None }) => {
            args.view = Some("network".into());
            false
        }
        Some(Command::Remote(extra)) => {
            anyhow::bail!(
                "Unexpected argument '{}' after the SSH destination",
                extra[0]
            )
        }
        Some(_) => true,
    };
    anyhow::ensure!(
        !unsupported
            && !args.snapshot
            && args.samples.is_none()
            && args.export_svg.is_none()
            && args.csv.is_none()
            && args.record.is_none()
            && args.replay.is_none()
            && !args.write_config
            && args.select_gpu.is_none(),
        "An SSH destination opens the remote interactive monitor only. Run other commands through ssh, for example: {target} rstop snapshot"
    );
    anyhow::ensure!(
        args.config.is_none(),
        "--config names a local file; remote rstop reads the configuration on {}",
        target.destination
    );
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "Remote monitoring needs a terminal."
    );
    let build = Build::current()?;
    let ssh = Ssh::new(target);
    let result = monitor(&ssh, &build, &forwarded(&args));
    ssh.close();
    match result? {
        0 => Ok(()),
        code => process::exit(code),
    }
}

fn monitor(ssh: &Ssh, build: &Build, args: &[String]) -> Result<i32> {
    let launch = launch_command(&build.name, args);
    let mut code = ssh.interactive(&launch)?;
    if code == Some(MISSING) {
        let local = Platform::local();
        let remote = ssh.platform()?;
        anyhow::ensure!(
            remote == local,
            "rstop is not installed on {} ({remote}); this build is for {local}. Install it there, then retry:\n  {} \"{INSTALLER}\"",
            ssh.target.destination,
            ssh.target
        );
        eprintln!(
            "rstop is not installed on {}; copying this build ({}) to ~/.cache/rstop there.",
            ssh.target.destination,
            rstop::model::bytes(build.bytes.len() as f64)
        );
        ssh.upload(build)?;
        code = ssh.interactive(&launch)?;
        anyhow::ensure!(
            code != Some(MISSING),
            "The copied rstop could not be started on {}",
            ssh.target.destination
        );
    }
    match code {
        Some(255) => anyhow::bail!("SSH connection to {} failed", ssh.target.destination),
        Some(code) => Ok(code),
        None if STOP.load(Ordering::Relaxed) => Ok(130),
        None => anyhow::bail!("ssh ended without an exit status"),
    }
}

/// Options that change the remote monitor's display or permissions.
fn forwarded(args: &Args) -> Vec<String> {
    let mut forwarded = Vec::new();
    for (name, value) in [
        ("--interval", args.interval.map(|v| v.to_string())),
        ("--theme", args.theme.clone()),
        ("--view", args.view.clone()),
        (
            "--gpu-panel",
            (args.gpu_panel != "summary").then(|| args.gpu_panel.clone()),
        ),
        ("--layout", args.layout.clone()),
    ] {
        if let Some(value) = value {
            forwarded.extend([name.into(), value]);
        }
    }
    for (name, set) in [
        ("--no-gpu", args.no_gpu),
        ("--ascii", args.ascii),
        ("--no-color", args.no_color),
        ("--threads", args.threads),
        ("--process-network", args.process_network),
        ("--read-only", args.read_only),
        ("--full-sampling", args.full_sampling),
        ("--no-config-exec", args.no_config_exec),
    ] {
        if set {
            forwarded.push(name.into());
        }
    }
    forwarded
}

/// An OpenSSH destination with an optional port taken from `host:port`.
#[derive(Debug, PartialEq)]
struct Target {
    destination: String,
    port: Option<u16>,
}

/// The equivalent OpenSSH command line, for messages.
impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.port {
            Some(port) => write!(f, "ssh -p {port} {}", self.destination),
            None => write!(f, "ssh {}", self.destination),
        }
    }
}

impl Target {
    fn parse(value: &str) -> Result<Self> {
        anyhow::ensure!(
            !value.is_empty()
                && !value.starts_with('-')
                && !value.chars().any(|c| c.is_whitespace() || c.is_control()),
            "Invalid SSH destination '{value}'"
        );
        // OpenSSH parses ssh:// URLs, including their ports, itself.
        if value.starts_with("ssh://") {
            return Ok(Self {
                destination: value.into(),
                port: None,
            });
        }
        let (user, host) = match value.rsplit_once('@') {
            Some((user, host)) => (Some(user), host),
            None => (None, value),
        };
        let port = |text: &str| {
            text.parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .with_context(|| format!("Invalid SSH port in '{value}'"))
        };
        let (host, port) = if let Some(rest) = host.strip_prefix('[') {
            let (inner, tail) = rest
                .split_once(']')
                .with_context(|| format!("Invalid SSH destination '{value}'"))?;
            match tail {
                "" => (inner, None),
                tail => (inner, Some(port(tail.strip_prefix(':').unwrap_or("-"))?)),
            }
        } else {
            match host.split_once(':') {
                // More than one colon is a bare IPv6 address.
                Some((name, text)) if !text.contains(':') => (name, Some(port(text)?)),
                _ => (host, None),
            }
        };
        anyhow::ensure!(!host.is_empty(), "Invalid SSH destination '{value}'");
        Ok(Self {
            destination: user.map_or_else(|| host.into(), |user| format!("{user}@{host}")),
            port,
        })
    }
}

struct Ssh {
    target: Target,
    /// Shared connection socket, so one password unlocks every step.
    control: Option<PathBuf>,
}

impl Ssh {
    fn new(target: Target) -> Self {
        Self {
            target,
            control: control_path(),
        }
    }

    fn command(&self) -> process::Command {
        let mut command = process::Command::new("ssh");
        if let Some(port) = self.target.port {
            command.arg("-p").arg(port.to_string());
        }
        if let Some(path) = &self.control {
            command
                .args(["-o", "ControlMaster=auto", "-o"])
                .arg(format!("ControlPath={}", path.display()))
                .args(["-o", "ControlPersist=60"]);
        }
        command
    }

    fn remote(&self, terminal: bool, script: &str) -> process::Command {
        let mut command = self.command();
        command
            .arg(if terminal { "-t" } else { "-T" })
            .arg("--")
            .arg(&self.target.destination)
            .arg(script);
        command
    }

    fn interactive(&self, script: &str) -> Result<Option<i32>> {
        Ok(self
            .remote(true, script)
            .status()
            .context("Cannot start ssh; install an OpenSSH client")?
            .code())
    }

    fn platform(&self) -> Result<Platform> {
        let output = self
            .remote(false, "uname -s; uname -m")
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .context("Cannot start ssh")?;
        anyhow::ensure!(
            output.status.success(),
            "Cannot identify the platform of {}",
            self.target.destination
        );
        let text = String::from_utf8_lossy(&output.stdout);
        // Login scripts may print banners first; uname output comes last.
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        match lines[..] {
            [.., os, arch] => Ok(Platform::from_uname(os, arch)),
            _ => anyhow::bail!(
                "Cannot identify the platform of {}",
                self.target.destination
            ),
        }
    }

    fn upload(&self, build: &Build) -> Result<()> {
        let mut child = self
            .remote(
                false,
                &format!("sh -c {}", quote(&upload_script(&build.name))),
            )
            .stdin(Stdio::piped())
            .spawn()
            .context("Cannot start ssh")?;
        let written = child
            .stdin
            .take()
            .expect("piped ssh input")
            .write_all(&build.bytes);
        let status = child.wait()?;
        anyhow::ensure!(
            status.success() && written.is_ok(),
            "Copying rstop to {} failed",
            self.target.destination
        );
        Ok(())
    }

    fn close(&self) {
        if let Some(path) = &self.control
            && path.exists()
        {
            let _ = self
                .command()
                .args(["-O", "exit", "--"])
                .arg(&self.target.destination)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

/// A private per-process socket path; shared /tmp directories are rejected
/// unless they belong to this user and are closed to others.
#[cfg(unix)]
fn control_path() -> Option<PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let uid = unsafe { libc::geteuid() };
    // Unix socket paths are short; avoid long per-user temporary directories.
    let dir = PathBuf::from(format!("/tmp/rstop-{uid}"));
    let _ = std::fs::DirBuilder::new().mode(0o700).create(&dir);
    let metadata = std::fs::symlink_metadata(&dir).ok()?;
    (metadata.is_dir() && metadata.uid() == uid && metadata.mode() & 0o077 == 0)
        .then(|| dir.join(process::id().to_string()))
}

/// Windows OpenSSH does not support connection sharing.
#[cfg(not(unix))]
fn control_path() -> Option<PathBuf> {
    None
}

#[derive(Debug, PartialEq)]
struct Platform {
    os: String,
    arch: String,
}

impl Platform {
    fn local() -> Self {
        Self {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        }
    }

    fn from_uname(os: &str, arch: &str) -> Self {
        Self {
            os: match os {
                "Linux" => "linux".into(),
                "Darwin" => "macos".into(),
                other => other.to_lowercase(),
            },
            arch: match arch {
                "arm64" => "aarch64".into(),
                "amd64" => "x86_64".into(),
                other => other.into(),
            },
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.os, self.arch)
    }
}

/// The running executable and its remote cache name.
struct Build {
    name: String,
    bytes: Vec<u8>,
}

impl Build {
    fn current() -> Result<Self> {
        // /proc/self/exe stays readable after an update replaces the file.
        #[cfg(target_os = "linux")]
        let path = PathBuf::from("/proc/self/exe");
        #[cfg(not(target_os = "linux"))]
        let path = std::env::current_exe().context("Cannot locate the rstop executable")?;
        let bytes =
            std::fs::read(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        let mut hasher = std::hash::DefaultHasher::new();
        hasher.write(&bytes);
        Ok(Self {
            name: format!(
                "rstop-{}-{:016x}",
                env!("CARGO_PKG_VERSION"),
                hasher.finish()
            ),
            bytes,
        })
    }
}

// Remote scripts run through the login shell, which may be sh, bash, zsh,
// fish or csh. Keep them on one line without single quotes, backslashes or `!`.

/// Exec the remote PATH's rstop, a common install location, or the upload cache.
fn launch_script(name: &str) -> String {
    format!(
        "d=${{XDG_CACHE_HOME:-$HOME/.cache}}/rstop; for r in \"$(command -v rstop 2>/dev/null)\" \"$HOME/.local/bin/rstop\" \"$HOME/.cargo/bin/rstop\" /opt/homebrew/bin/rstop /usr/local/bin/rstop \"$d/{name}\"; do if [ -f \"$r\" ] && [ -x \"$r\" ]; then exec \"$r\" \"$@\"; fi; done; exit {MISSING}"
    )
}

fn launch_command(name: &str, args: &[String]) -> String {
    std::iter::once(format!("sh -c {} rstop", quote(&launch_script(name))))
        .chain(args.iter().map(|arg| quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Store standard input as the private cached build, replacing older copies.
fn upload_script(name: &str) -> String {
    format!(
        "umask 077; d=${{XDG_CACHE_HOME:-$HOME/.cache}}/rstop; t=$d/.upload-$$; mkdir -p \"$d\" && cat > \"$t\" && chmod 700 \"$t\" && rm -f \"$d\"/rstop-* && mv -f \"$t\" \"$d/{name}\" || {{ rm -f \"$t\"; exit 1; }}"
    )
}

fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:@,+%".contains(&b))
    {
        value.into()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(destination: &str, port: Option<u16>) -> Target {
        Target {
            destination: destination.into(),
            port,
        }
    }

    #[test]
    fn destinations_accept_ssh_forms_and_host_ports() {
        for (input, expected) in [
            ("gpu-box", target("gpu-box", None)),
            ("alice@10.0.0.2", target("alice@10.0.0.2", None)),
            ("alice@10.0.0.2:2222", target("alice@10.0.0.2", Some(2222))),
            ("root@[fe80::1]:22", target("root@fe80::1", Some(22))),
            ("[::1]", target("::1", None)),
            ("bob@fe80::1", target("bob@fe80::1", None)),
            ("me@corp@jump", target("me@corp@jump", None)),
            (
                "ssh://a@b.example:2200",
                target("ssh://a@b.example:2200", None),
            ),
        ] {
            assert_eq!(Target::parse(input).unwrap(), expected, "{input}");
        }
        assert_eq!(
            Target::parse("alice@box:2222").unwrap().to_string(),
            "ssh -p 2222 alice@box"
        );
        for input in [
            "",
            "-oProxyCommand=touch",
            "alice@",
            "host:",
            "host:ssh",
            "host:0",
            "host:70000",
            "[::1]x",
            "[::1",
            "a b",
        ] {
            assert!(Target::parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn uname_output_maps_to_rust_platform_names() {
        assert_eq!(
            Platform::from_uname("Darwin", "arm64"),
            Platform {
                os: "macos".into(),
                arch: "aarch64".into()
            }
        );
        assert_eq!(
            Platform::from_uname("Linux", "x86_64").to_string(),
            "linux x86_64"
        );
    }

    #[test]
    fn remote_scripts_survive_login_shell_quoting() {
        for script in [launch_script("rstop-x"), upload_script("rstop-x")] {
            assert!(!script.contains(['\n', '\'', '\\', '!']), "{script}");
        }
        assert_eq!(quote("--theme"), "--theme");
        assert_eq!(quote(""), "''");
        assert_eq!(quote("it's here"), r"'it'\''s here'");
    }

    #[cfg(unix)]
    mod shell {
        use super::super::*;
        use std::{fs, os::unix::fs::PermissionsExt, path::Path};

        struct Home(PathBuf);

        impl Home {
            fn new(name: &str) -> Self {
                let path =
                    std::env::temp_dir().join(format!("rstop-remote-{name}-{}", process::id()));
                let _ = fs::remove_dir_all(&path);
                fs::create_dir_all(&path).unwrap();
                Self(path)
            }

            fn executable(&self, relative: &str) {
                let path = self.0.join(relative);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, "#!/bin/sh\nprintf '%s|' \"$0\" \"$@\"\n").unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }

            fn run(&self, command: &str, input: &[u8]) -> process::Output {
                let mut child = process::Command::new("/bin/sh")
                    .arg("-c")
                    .arg(command)
                    .env_clear()
                    .env("HOME", &self.0)
                    .env("PATH", "/usr/bin:/bin")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                child.stdin.take().unwrap().write_all(input).unwrap();
                child.wait_with_output().unwrap()
            }
        }

        impl Drop for Home {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        /// Hosts with a system-wide rstop cannot exercise the missing/cache path.
        fn system_rstop() -> bool {
            [
                "/usr/bin/rstop",
                "/bin/rstop",
                "/opt/homebrew/bin/rstop",
                "/usr/local/bin/rstop",
            ]
            .into_iter()
            .any(|path| Path::new(path).exists())
        }

        #[test]
        fn launch_prefers_installed_rstop_and_forwards_quoted_arguments() {
            let home = Home::new("installed");
            home.executable(".local/bin/rstop");
            home.executable(".cache/rstop/rstop-x");
            let args = ["--theme".to_string(), "it's two".into()];
            let output = home.run(&launch_command("rstop-x", &args), b"");
            assert!(output.status.success());
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{}/.local/bin/rstop|--theme|it's two|", home.0.display())
            );
        }

        #[test]
        fn launch_uses_the_cached_build_or_reports_it_missing() {
            if system_rstop() {
                return;
            }
            let home = Home::new("cache");
            let output = home.run(&launch_command("rstop-x", &[]), b"");
            assert_eq!(output.status.code(), Some(MISSING));
            home.executable(".cache/rstop/rstop-x");
            let output = home.run(&launch_command("rstop-x", &["--ascii".into()]), b"");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{}/.cache/rstop/rstop-x|--ascii|", home.0.display())
            );
        }

        #[test]
        fn upload_replaces_older_builds_with_a_private_executable() {
            let home = Home::new("upload");
            home.executable(".cache/rstop/rstop-old");
            let command = format!("sh -c {}", quote(&upload_script("rstop-new")));
            let output = home.run(&command, b"binary\0bytes");
            assert!(output.status.success());
            let dir = home.0.join(".cache/rstop");
            assert!(!dir.join("rstop-old").exists());
            let uploaded = dir.join("rstop-new");
            assert_eq!(fs::read(&uploaded).unwrap(), b"binary\0bytes");
            let mode = fs::metadata(&uploaded).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
            assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        }
    }
}
