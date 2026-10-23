//! On-demand log sources. Never read another process's pipe, socket or terminal.
use crate::model::{Container, Process, Service, Snapshot};
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::Duration,
};

const TAIL_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Target {
    Command {
        title: String,
        program: String,
        args: Vec<String>,
    },
    Process {
        pid: u32,
        start: u64,
        containers: Vec<Container>,
        service: Option<Box<Service>>,
    },
    File(PathBuf),
}

impl Target {
    pub fn process(process: &Process, snapshot: &Snapshot) -> Self {
        let mut pid = process.pid;
        let mut service = None;
        // A service may launch worker processes; choose the nearest known owner.
        for _ in 0..32 {
            service = snapshot
                .inventory
                .services
                .iter()
                .find(|s| s.pid == Some(pid))
                .cloned();
            if service.is_some() {
                break;
            }
            let Some(parent) = snapshot
                .processes
                .iter()
                .find(|p| p.pid == pid)
                .and_then(|p| p.parent)
            else {
                break;
            };
            if parent <= 1 || parent == pid {
                break;
            }
            pid = parent;
        }
        Self::Process {
            pid: process.pid,
            start: process.start_time,
            containers: snapshot.inventory.containers.clone(),
            service: service.map(Box::new),
        }
    }

    pub fn container(container: &Container, lines: usize) -> Self {
        Self::Command {
            title: format!("LOGS {} · {}", container.name, container.runtime),
            program: container.runtime.clone(),
            args: vec![
                "logs".into(),
                "--tail".into(),
                lines.to_string(),
                "--timestamps".into(),
                container.id.clone(),
            ],
        }
    }

    pub fn service(service: &Service, lines: usize) -> Option<Self> {
        #[cfg(target_os = "macos")]
        let _ = lines;
        #[cfg(target_os = "linux")]
        let (program, args) = (
            "journalctl",
            vec![
                format!("--{}", service.scope),
                "--unit".into(),
                service.name.clone(),
                "--boot".into(),
                "--quiet".into(),
                "--lines".into(),
                lines.to_string(),
                "--no-pager".into(),
                "--output=short-iso".into(),
            ],
        );
        #[cfg(target_os = "macos")]
        let (program, args) = (
            "log",
            vec![
                "show".into(),
                "--last".into(),
                "10m".into(),
                "--style".into(),
                "compact".into(),
                "--predicate".into(),
                format!(
                    "eventMessage CONTAINS \"{}\"",
                    service.name.replace(['"', '\\'], "")
                ),
            ],
        );
        #[cfg(windows)]
        let (program, args) = (
            "powershell.exe",
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                format!(
                    "Get-WinEvent -FilterHashtable @{{LogName='System';ProviderName='Service Control Manager'}} -MaxEvents {} | Where-Object {{$_.Message -match [regex]::Escape('{}')}} | Format-List TimeCreated,Id,LevelDisplayName,Message",
                    lines,
                    service.name.replace(char::from(39), "''")
                ),
            ],
        );
        #[cfg(any(target_os = "linux", target_os = "macos", windows))]
        {
            Some(Self::Command {
                title: format!("JOURNAL {}", service.name),
                program: program.into(),
                args,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            let _ = (service, lines);
            None
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::Command { title, .. } => title.clone(),
            Self::Process { pid, .. } => format!("OUTPUT · PID {pid}"),
            Self::File(path) => format!(
                "OUTPUT · {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
        }
    }

    pub fn read(&self, lines: usize) -> Result<String> {
        let lines = lines.clamp(1, 100_000);
        let body = match self {
            Self::Command { program, args, .. } => {
                let args: Vec<_> = args.iter().map(String::as_str).collect();
                let output =
                    crate::discovery::command_with_stderr(program, &args, Duration::from_secs(4))?;
                let output = limit_command_output(program, &args, &output, lines);
                let empty = if output.trim().is_empty() {
                    "No saved log entries are available."
                } else {
                    output
                };
                format!("Source: {}\n\n{}", self.title(), empty)
            }
            Self::File(path) => {
                format!("Source: {}\n\n{}", path.display(), read_tail(path, lines)?)
            }
            Self::Process {
                pid,
                start,
                containers,
                service,
            } => {
                validate(*pid, *start)?;
                let result = process_output(*pid, *start, containers, service.as_deref(), lines)?;
                validate(*pid, *start)?;
                result
            }
        };
        Ok(clean(&body))
    }
}

fn limit_command_output<'a>(
    program: &str,
    args: &[&str],
    output: &'a str,
    lines: usize,
) -> &'a str {
    // Other log sources already use --lines, --tail, or -MaxEvents. In particular,
    // Windows events are newest first and Format-List expands each event into multiple lines.
    if program == "log" && args.first() == Some(&"show") {
        tail_lines(output, lines)
    } else {
        output
    }
}

/// macOS `log show` has no native line limit and returns entries oldest first.
fn tail_lines(text: &str, lines: usize) -> &str {
    match text
        .trim_end_matches('\n')
        .rmatch_indices('\n')
        .nth(lines.saturating_sub(1))
    {
        Some((index, _)) => &text[index + 1..],
        None => text,
    }
}

fn validate(pid: u32, start: u64) -> Result<()> {
    let id = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[id]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    let process = system
        .process(id)
        .context("Process exited; output monitoring stopped")?;
    anyhow::ensure!(
        process.start_time() == start,
        "PID identity changed; output monitoring stopped"
    );
    Ok(())
}

/// A separate open file description preserves the writer's offset, including deleted files.
pub fn read_tail(path: &Path, lines: usize) -> Result<String> {
    anyhow::ensure!(
        fs::metadata(path)?.is_file(),
        "Not a regular log file; pipes, devices and terminals are not read"
    );
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file(),
        "Log source changed to a non-regular file"
    );
    let start = metadata.len().saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut data = Vec::new();
    file.take(metadata.len() - start).read_to_end(&mut data)?;
    let text = String::from_utf8_lossy(&data);
    let text = if start > 0 {
        text.split_once('\n')
            .map_or(text.as_ref(), |(_, tail)| tail)
    } else {
        &text
    };
    let skip = text.lines().count().saturating_sub(lines.clamp(1, 100_000));
    let tail = text.lines().skip(skip).collect::<Vec<_>>().join("\n");
    Ok(format!(
        "{}{}",
        if start > 0 {
            "[Tail limited to 1 MiB]\n"
        } else {
            ""
        },
        tail
    ))
}

#[cfg(target_os = "linux")]
fn process_output(
    pid: u32,
    start: u64,
    containers: &[Container],
    known_service: Option<&Service>,
    lines: usize,
) -> Result<String> {
    use std::os::unix::fs::MetadataExt;
    let mut descriptions = vec![];
    let mut files = vec![];
    let mut identities = vec![];
    for (fd, label) in [(1, "stdout"), (2, "stderr")] {
        let path = PathBuf::from(format!("/proc/{pid}/fd/{fd}"));
        let link = fs::read_link(&path);
        let name = link
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| format!("unavailable ({e})"));
        if let Ok(metadata) = fs::metadata(&path)
            && metadata.is_file()
        {
            let identity = (metadata.dev(), metadata.ino());
            descriptions.push(format!("{label} → {name} (log file)"));
            if identities.contains(&identity) {
                continue;
            }
            identities.push(identity);
            files.push(format!(
                "── {label}: {name} ──\n{}",
                read_tail(&path, lines).unwrap_or_else(|e| format!("Cannot read file: {e}"))
            ));
        } else {
            descriptions.push(format!("{label} → {name} (not a readable log file)"));
        }
    }
    let summary = descriptions.join("\n");
    if !files.is_empty() {
        return Ok(format!("{summary}\n\n{}", files.join("\n\n")));
    }
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default();
    if let Some(container) = containers
        .iter()
        .find(|c| c.id.len() >= 12 && cgroup.contains(&c.id))
    {
        let mut target = Target::container(container, lines);
        if let Target::Command { args, .. } = &mut target {
            args.splice(1..1, ["--since".into(), start.to_string()]);
        }
        return Ok(format!(
            "{summary}\n\nContainer output (all processes in container):\n{}",
            target.read(lines)?
        ));
    }
    let service = cgroup_service(&cgroup).or_else(|| known_service.cloned());
    if let Some(mut target) = service.as_ref().and_then(|s| Target::service(s, lines)) {
        if let Target::Command { args, .. } = &mut target {
            args.extend(["--since".into(), format!("@{start}")]);
        }
        return Ok(format!(
            "{summary}\n\nService journal (whole unit, since process start):\n{}",
            target.read(lines)?
        ));
    }
    let target = Target::Command {
        title: format!("JOURNAL PID {pid} · current boot / process lifetime"),
        program: "journalctl".into(),
        args: vec![
            "--boot".into(),
            "--quiet".into(),
            "--no-pager".into(),
            "--output=short-iso".into(),
            "--lines".into(),
            lines.to_string(),
            "--since".into(),
            format!("@{start}"),
            format!("_PID={pid}"),
        ],
    };
    let journal = target
        .read(lines)
        .unwrap_or_else(|e| format!("Journal unavailable: {e}"));
    Ok(format!(
        "{summary}\n\n{journal}\n\nTerminal/pipe output is not retained automatically.\nPress p to follow a log file. Past unsaved output cannot be recovered."
    ))
}

#[cfg(target_os = "linux")]
fn cgroup_service(cgroup: &str) -> Option<Service> {
    cgroup.lines().find_map(|line| {
        let path = line.splitn(3, ':').nth(2)?;
        let name = path
            .split('/')
            .rev()
            .find(|part| part.ends_with(".service"))?;
        let user = path.contains("/user@") && !name.starts_with("user@");
        if user && !path.contains(&format!("/user@{}.service/", unsafe { libc::geteuid() })) {
            // --user addresses our own journal; never attribute another user's unit to it.
            return None;
        }
        Some(Service {
            name: name.into(),
            scope: if user { "user" } else { "system" }.into(),
            ..Service::default()
        })
    })
}

#[cfg(target_os = "macos")]
fn process_output(
    pid: u32,
    _start: u64,
    _containers: &[Container],
    _service: Option<&Service>,
    lines: usize,
) -> Result<String> {
    // lsof performs descriptor discovery; only regular files are opened by rstop.
    let output = crate::discovery::command(
        "lsof",
        &["-nP", "-a", "-p", &pid.to_string(), "-d", "1,2", "-Fftn"],
        Duration::from_secs(2),
    )?;
    let mut regular = false;
    let mut files = vec![];
    for line in output.lines() {
        if line.starts_with('f') {
            regular = false;
        }
        if line == "tREG" {
            regular = true;
        }
        if regular && let Some(path) = line.strip_prefix('n') {
            files.push(format!(
                "Source: {path}\n\n{}",
                read_tail(Path::new(path), lines).unwrap_or_else(|e| e.to_string())
            ));
        }
    }
    if !files.is_empty() {
        return Ok(files.join("\n\n"));
    }
    Ok("No readable stdout/stderr log file found.\nTerminal/pipe output cannot be recovered automatically.\nPress p to follow an application log file.".into())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_output(
    _pid: u32,
    _start: u64,
    _containers: &[Container],
    service: Option<&Service>,
    lines: usize,
) -> Result<String> {
    let mut body = "Automatic stdout/stderr discovery is unavailable on this OS.\nPress p to follow an application log file.".to_string();
    if let Some(target) = service.and_then(|s| Target::service(s, lines)) {
        body.push_str(&format!(
            "\n\nService manager events (not stdout/stderr):\n{}",
            target.read(lines)?
        ));
    }
    Ok(body)
}

fn clean(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut output = String::new();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.next() == Some('\\')) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if !c.is_control() || c == '\n' || c == '\t' {
            output.push(c);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_log_limits_preserve_newest_first_multiline_events() {
        let text =
            "TimeCreated : 12:00\nMessage : NEWEST\n\nTimeCreated : 11:00\nMessage : OLDER\n";
        for (program, args) in [
            ("powershell.exe", vec!["-Command", "Get-WinEvent"]),
            ("journalctl", vec!["--lines", "2"]),
            ("docker", vec!["logs", "--tail", "2"]),
            ("podman", vec!["logs", "--tail", "2"]),
        ] {
            assert_eq!(super::limit_command_output(program, &args, text, 2), text);
        }
    }

    #[test]
    fn macos_log_show_is_limited_without_changing_other_commands() {
        let text = "old\nnewer\nnewest\n";
        assert_eq!(
            super::limit_command_output("log", &["show", "--last", "10m"], text, 2),
            "newer\nnewest\n"
        );
        assert_eq!(
            super::limit_command_output("log", &["stream"], text, 2),
            text
        );
    }

    #[test]
    fn command_output_keeps_only_the_newest_lines() {
        assert_eq!(super::tail_lines("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(super::tail_lines("a\nb\nc", 1), "c");
        assert_eq!(super::tail_lines("a\nb", 5), "a\nb");
        assert_eq!(super::tail_lines("", 3), "");
    }

    use super::*;

    #[test]
    fn log_escape_sequences_cannot_change_terminal_state() {
        assert_eq!(
            clean("\x1b[31merror\x1b[0m\n\x1b]0;title\x07ok\0\tend"),
            "error\nok\tend"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn cgroups_resolve_nested_units_and_user_scope() {
        let system = cgroup_service("0::/system.slice/example.service/workers").unwrap();
        assert_eq!(system.name, "example.service");
        assert_eq!(system.scope, "system");
        let user = cgroup_service(&format!(
            "0::/user.slice/user-{}.slice/user@{}.service/app.slice/test.service",
            unsafe { libc::geteuid() },
            unsafe { libc::geteuid() }
        ))
        .unwrap();
        assert_eq!(user.name, "test.service");
        assert_eq!(user.scope, "user");
        assert!(
            cgroup_service("0::/user.slice/user@4294967294.service/app.slice/test.service")
                .is_none()
        );
        assert!(cgroup_service("0::/docker/1234567890").is_none());
    }
}
