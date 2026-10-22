//! Explicit ROS graph queries using a selected process's communication environment.
//! Sampling/classification never invokes these tools.
use crate::model::Process;
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Session {
    pub process: Process,
    pub version: u8,
    pub environment: BTreeMap<String, String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListKind {
    Topics,
    Nodes,
    Services,
    Parameters(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub types: Vec<String>,
}
impl Entry {
    pub fn display(&self) -> String {
        if self.types.is_empty() {
            self.name.clone()
        } else {
            format!("{} · {}", self.name, self.types.join(", "))
        }
    }
}

#[derive(Clone, Debug)]
pub enum Query {
    List(ListKind),
    TopicInfo(String),
    NodeInfo(String),
    ServiceInfo(String),
    Echo(String, bool),
    Hz(String),
    Definition(Entry),
    Parameter(String, String),
    Context,
}

#[derive(Debug)]
pub enum Reply {
    Entries(ListKind, Vec<Entry>),
    Report(String),
}

fn communication_key(key: &str) -> bool {
    key.starts_with("ROS_")
        || key.starts_with("RMW_")
        || key.starts_with("ZENOH_")
        || key.starts_with("CYCLONEDDS_")
        || key.starts_with("FASTDDS_")
        || key.starts_with("FASTRTPS_")
        || key == "RUST_LOG"
}

#[cfg(any(target_os = "linux", test))]
fn environment_key(key: &str) -> bool {
    communication_key(key)
        || [
            "PATH",
            "HOME",
            "USER",
            "LOGNAME",
            "TMPDIR",
            "LANG",
            "PYTHONPATH",
            "PYTHONHOME",
            "LD_LIBRARY_PATH",
            "AMENT_PREFIX_PATH",
            "COLCON_PREFIX_PATH",
            "CMAKE_PREFIX_PATH",
        ]
        .contains(&key)
}

#[cfg(any(target_os = "linux", test))]
fn parse_environment(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    ensure!(
        bytes.len() <= 256 * 1024,
        "Process environment exceeds the 256 KiB limit"
    );
    let mut values = BTreeMap::new();
    for value in bytes.split(|v| *v == 0) {
        let Some(eq) = value.iter().position(|v| *v == b'=') else {
            continue;
        };
        let key = std::str::from_utf8(&value[..eq])?;
        if environment_key(key) {
            values.insert(
                key.to_string(),
                std::str::from_utf8(&value[eq + 1..])?.to_string(),
            );
        }
    }
    Ok(values)
}

impl Session {
    pub fn from_process(process: Process) -> Result<Self> {
        crate::tags::check_identity(&process)?;
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let root = PathBuf::from(format!("/proc/{}", process.pid));
            ensure!(
                std::fs::metadata(&root)?.uid() == unsafe { libc::geteuid() },
                "Cannot use another user's ROS environment; run rstop as the ROS process owner"
            );
            for namespace in ["net", "mnt"] {
                ensure!(
                    std::fs::read_link(root.join(format!("ns/{namespace}")))?
                        == std::fs::read_link(format!("/proc/self/ns/{namespace}"))?,
                    "ROS process has a different {namespace} namespace. Run rstop inside its container / namespace to inspect that graph"
                );
            }
            let mut bytes = vec![];
            std::fs::File::open(root.join("environ"))
                .context("Cannot read the ROS process environment")?
                .take(256 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            let environment = parse_environment(&bytes)?;
            let version = match environment.get("ROS_VERSION").map(String::as_str) {
                Some("1") => 1,
                Some("2") => 2,
                _ if process
                    .command
                    .split_whitespace()
                    .any(|v| v == "--ros-args")
                    || [
                        "ros2",
                        "rviz2",
                        "component_container",
                        "component_container_mt",
                    ]
                    .contains(&process.name.as_str()) =>
                {
                    2
                }
                _ if [
                    "roscore",
                    "rosmaster",
                    "roslaunch",
                    "rosout",
                    "rostopic",
                    "rosnode",
                ]
                .contains(&process.name.as_str()) =>
                {
                    1
                }
                _ => bail!(
                    "Cannot determine ROS 1 / ROS 2 from this process. Launch the node from its sourced ROS environment and reopen this menu"
                ),
            };
            let cwd = std::fs::read_link(root.join("cwd"))?;
            crate::tags::check_identity(&process)?;
            Ok(Self {
                process,
                version,
                environment,
                cwd,
            })
        }
        #[cfg(not(target_os = "linux"))]
        bail!(
            "Process-scoped ROS tools currently require Linux /proc. Use ROS CLI in the node's sourced shell on this platform"
        )
    }

    pub fn scope(&self) -> String {
        if self.version == 2 {
            format!(
                "ROS 2 · Domain {}",
                self.environment
                    .get("ROS_DOMAIN_ID")
                    .map_or("0", String::as_str)
            )
        } else {
            format!(
                "ROS 1 · {}",
                self.environment
                    .get("ROS_MASTER_URI")
                    .map_or("http://localhost:11311", String::as_str)
            )
        }
    }

    pub fn context(&self) -> String {
        let mut report = format!(
            "{}\nEnvironment from {} · PID {}\n\nGraph queries cover this communication domain/master. A process can contain multiple nodes; graph entries are not PID attribution.\n\n",
            self.scope(),
            self.process.name,
            self.process.pid
        );
        for (key, value) in &self.environment {
            if communication_key(key) {
                report.push_str(&format!("{key}={value}\n"));
            }
        }
        if self.version == 2 {
            report.push_str("\nROS 2 queries use direct discovery; no CLI daemon is started.\n");
        }
        report
    }

    fn program(&self, name: &str) -> Result<PathBuf> {
        self.environment.get("PATH").into_iter().flat_map(|p| std::env::split_paths(p))
            .map(|p| if p.is_absolute() { p } else { self.cwd.join(p) })
            .map(|p| p.join(name)).find(|p| p.is_file())
            .with_context(|| format!("{name} is missing from PID {}'s PATH. Install its ROS CLI package and source the matching ROS / workspace setup before launching the node", self.process.pid))
    }

    pub fn query(&self, query: Query, cancel: &Arc<AtomicBool>) -> Result<Reply> {
        crate::tags::check_identity(&self.process)?;
        if let Query::Context = query {
            return Ok(Reply::Report(self.context()));
        }
        let mut report = format!(
            "{}\nEnvironment: PID {}\n\n",
            self.scope(),
            self.process.pid
        );
        let result = if let Query::Definition(entry) = &query {
            valid_name(&entry.name)?;
            let types = if entry.types.is_empty() {
                let (program, args) = if self.version == 2 {
                    (
                        "ros2",
                        vec![
                            "topic",
                            "type",
                            &entry.name,
                            "--no-daemon",
                            "--spin-time",
                            "2",
                        ],
                    )
                } else {
                    ("rostopic", vec!["type", &entry.name])
                };
                let output = self.run(program, &args, Duration::from_secs(8), false, cancel)?;
                output
                    .lines()
                    .filter(|line| valid_type(line).is_ok())
                    .map(String::from)
                    .collect::<Vec<_>>()
            } else {
                entry.types.clone()
            };
            ensure!(
                types.len() == 1,
                "Topic has {} message types; select a topic with one type to display its definition",
                types.len()
            );
            valid_type(&types[0])?;
            let (program, args) = if self.version == 2 {
                ("ros2", vec!["interface", "show", &types[0]])
            } else {
                ("rosmsg", vec!["show", &types[0]])
            };
            report.push_str(&format!("$ {program} {}\n\n", args.join(" ")));
            self.run(program, &args, Duration::from_secs(8), false, cancel)?
        } else {
            let spec = command_spec(self.version, &query)?;
            report.push_str(&format!("$ {} {}\n\n", spec.program, spec.args.join(" ")));
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();
            self.run(spec.program, &args, spec.timeout, spec.sample, cancel)?
        };
        crate::tags::check_identity(&self.process)?;
        if let Query::List(kind) = query {
            ensure!(
                !result.contains("Output truncated at"),
                "ROS list exceeded the output limit; use the ROS CLI to inspect this large graph"
            );
            return Ok(Reply::Entries(kind.clone(), parse_entries(&kind, &result)));
        }
        if let Query::ServiceInfo(name) = &query
            && self.version == 2
        {
            let entry = parse_entries(&ListKind::Services, &result)
                .into_iter()
                .find(|e| &e.name == name)
                .context("Service no longer visible in this domain")?;
            report.push_str(&entry.display());
        } else {
            report.push_str(&result);
        }
        if let Query::Hz(_) = query {
            report.push_str("\nEight-second subscriber sample; observed receive rate depends on discovery, QoS and host load.\n");
            if !result.contains("average rate:") {
                report.push_str("No rate estimate yet; discovery, low message frequency or incompatible QoS can leave this sample without enough messages.\n");
            }
        }
        Ok(Reply::Report(report))
    }

    fn run(
        &self,
        program: &str,
        args: &[&str],
        timeout: Duration,
        sample: bool,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String> {
        let mut command = Command::new(self.program(program)?);
        command
            .args(args)
            .env_clear()
            .envs(&self.environment)
            .current_dir(&self.cwd)
            .env("PYTHONUNBUFFERED", "1")
            .env("LC_ALL", "C")
            .env("NO_COLOR", "1");
        capture(command, timeout, sample, cancel)
    }
}

pub struct CommandSpec {
    pub program: &'static str,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub sample: bool,
}

fn valid_name(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 1024
            && value.starts_with('/')
            && value != "/"
            && value.split('/').skip(1).all(|part| !part.is_empty()
                && !part.starts_with(|c: char| c.is_ascii_digit())
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')),
        "Invalid ROS name: use a fully qualified name such as /robot/chatter"
    );
    Ok(())
}
fn valid_type(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 1024
            && (2..=3).contains(&value.split('/').count())
            && value.split('/').all(|part| !part.is_empty()
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')),
        "Invalid ROS interface type"
    );
    Ok(())
}
fn valid_parameter(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 1024
            && !value.starts_with('-')
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ['_', '.', '/'].contains(&c)),
        "Invalid ROS parameter name"
    );
    Ok(())
}

/// Construct fixed read/subscribe commands. User/graph values remain individual argv entries.
pub fn command_spec(version: u8, query: &Query) -> Result<CommandSpec> {
    use Query::*;
    ensure!([1, 2].contains(&version), "Unknown ROS version");
    match query {
        TopicInfo(n) | NodeInfo(n) | ServiceInfo(n) | Echo(n, _) | Hz(n) => valid_name(n)?,
        List(ListKind::Parameters(n)) if version == 2 => valid_name(n)?,
        Parameter(node, name) => {
            if version == 2 {
                valid_name(node)?;
            }
            valid_parameter(name)?;
        }
        _ => {}
    }
    let (program, args): (&'static str, Vec<&str>) = match (version, query) {
        (2, List(ListKind::Topics)) => ("ros2", vec!["topic", "list", "-t"]),
        (2, List(ListKind::Nodes)) => ("ros2", vec!["node", "list"]),
        (2, List(ListKind::Services)) => ("ros2", vec!["service", "list", "-t"]),
        (2, List(ListKind::Parameters(node))) => ("ros2", vec!["param", "list", node]),
        (2, TopicInfo(n)) => ("ros2", vec!["topic", "info", n, "--verbose"]),
        (2, NodeInfo(n)) => ("ros2", vec!["node", "info", n]),
        (2, ServiceInfo(_)) => ("ros2", vec!["service", "list", "-t"]),
        (2, Echo(n, true)) => ("ros2", vec!["topic", "echo", n, "--once"]),
        (2, Echo(n, false)) => ("ros2", vec!["topic", "echo", n]),
        (2, Hz(n)) => (
            "ros2",
            vec![
                "topic",
                "hz",
                n,
                "--window",
                "100",
                "--wall-time",
                "--spin-time",
                "2",
            ],
        ),
        (2, Parameter(node, name)) => ("ros2", vec!["param", "get", node, name]),
        (1, List(ListKind::Topics)) => ("rostopic", vec!["list"]),
        (1, List(ListKind::Nodes)) => ("rosnode", vec!["list"]),
        (1, List(ListKind::Services)) => ("rosservice", vec!["list"]),
        (1, List(ListKind::Parameters(_))) => ("rosparam", vec!["list"]),
        (1, TopicInfo(n)) => ("rostopic", vec!["info", n]),
        (1, NodeInfo(n)) => ("rosnode", vec!["info", n]),
        (1, ServiceInfo(n)) => ("rosservice", vec!["info", n]),
        (1, Echo(n, true)) => ("rostopic", vec!["echo", "-n", "1", n]),
        (1, Echo(n, false)) => ("rostopic", vec!["echo", n]),
        (1, Hz(n)) => ("rostopic", vec!["hz", "-w", "100", n]),
        (1, Parameter(_, name)) => ("rosparam", vec!["get", name]),
        _ => bail!("Unsupported ROS query"),
    };
    let mut args = args.into_iter().map(String::from).collect::<Vec<_>>();
    // hz uses DirectNode internally and does not accept --no-daemon.
    if version == 2 && !matches!(query, Hz(_)) {
        args.extend(["--no-daemon", "--spin-time", "2"].map(String::from));
    }
    let sample = matches!(query, Hz(_) | Echo(_, false));
    Ok(CommandSpec {
        program,
        args,
        timeout: Duration::from_secs(if matches!(query, Echo(_, false)) {
            5
        } else {
            8
        }),
        sample,
    })
}

pub fn parse_entries(kind: &ListKind, output: &str) -> Vec<Entry> {
    let mut entries = BTreeMap::<String, Vec<String>>::new();
    for line in output.lines().take(16_384) {
        let line = line.trim();
        if line == "CLI diagnostics:" {
            break;
        }
        let (name, suffix) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if if matches!(kind, ListKind::Parameters(_)) {
            valid_parameter(name).is_err()
        } else {
            valid_name(name).is_err()
        } {
            continue;
        }
        // param list may emit a node heading when a distro returns grouped output.
        if name.ends_with(':') {
            continue;
        }
        if matches!(kind, ListKind::Parameters(_)) && !suffix.is_empty() {
            continue;
        }
        let types = suffix
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(str::trim)
            .filter(|t| valid_type(t).is_ok())
            .map(String::from)
            .collect();
        entries.entry(name.into()).or_insert(types);
        if entries.len() >= 4096 {
            break;
        }
    }
    entries
        .into_iter()
        .map(|(name, types)| Entry { name, types })
        .collect()
}

fn capture(
    mut command: Command,
    timeout: Duration,
    sample: bool,
    cancel: &Arc<AtomicBool>,
) -> Result<String> {
    ensure!(!cancel.load(Ordering::Relaxed), "ROS query canceled");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot start ROS CLI in the selected process's environment")?;
    #[cfg(windows)]
    let job = match crate::windows_inspect::QueryJob::attach(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let limit = Arc::new(AtomicBool::new(false));
    let readers = [
        child
            .stdout
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
    ];
    let readers = readers
        .into_iter()
        .flatten()
        .map(|mut reader| {
            let limit = limit.clone();
            thread::spawn(move || {
                let mut result = vec![];
                let mut chunk = [0; 8192];
                while let Ok(n) = reader.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let keep = n.min((256 * 1024usize).saturating_sub(result.len()));
                    result.extend_from_slice(&chunk[..keep]);
                    if keep < n {
                        limit.store(true, Ordering::Relaxed);
                    }
                }
                result
            })
        })
        .collect::<Vec<_>>();
    let started = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed)
            || crate::STOP.load(Ordering::Relaxed)
            || limit.load(Ordering::Relaxed)
            || started.elapsed() >= timeout
        {
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        thread::sleep(Duration::from_millis(20));
    };
    // Also reap descendants which inherited the output pipes.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    if status.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    #[cfg(windows)]
    drop(job);
    let mut output = readers
        .into_iter()
        .map(|r| String::from_utf8_lossy(&r.join().unwrap_or_default()).into_owned());
    let stdout = output.next().unwrap_or_default();
    let stderr = output.next().unwrap_or_default();
    ensure!(!cancel.load(Ordering::Relaxed), "ROS query canceled");
    let truncated = limit.load(Ordering::Relaxed);
    if !truncated && let Some(status) = status {
        ensure!(
            status.success(),
            "ROS CLI failed: {}",
            terminal_text(&stderr)
        );
    } else if !sample && !truncated {
        bail!(
            "ROS query timed out ({} s). {}\nCheck Domain / master, middleware, QoS and message packages in the selected process's environment",
            timeout.as_secs(),
            terminal_text(&stderr)
        );
    }
    let mut result = terminal_text(&stdout);
    if sample && stdout.trim().is_empty() {
        result.push_str(
            "No samples received before the capture ended. Check publishers, discovery and QoS.\n",
        );
    }
    if !stderr.trim().is_empty() {
        result.push_str(&format!("\nCLI diagnostics:\n{}", terminal_text(&stderr)));
    }
    if truncated {
        result.push_str("\nOutput truncated at 256 KiB; capture stopped.\n");
    } else if sample {
        result.push_str("\nTimed sample completed.\n");
    }
    Ok(result)
}

fn terminal_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || ['\n', '\t'].contains(c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_is_bounded_and_copies_only_ros_and_runtime_paths() {
        let values = parse_environment(b"ROS_VERSION=2\0ROS_DOMAIN_ID=43\0RMW_IMPLEMENTATION=rmw_cyclonedds_cpp\0PATH=/opt/ros/humble/bin:/usr/bin\0PYTHONPATH=/workspace/install/lib/python3/dist-packages\0LD_PRELOAD=/tmp/custom.so\0API_TOKEN=secret\0HTTPS_PROXY=http://proxy\0").unwrap();
        assert_eq!(values["ROS_DOMAIN_ID"], "43");
        assert!(values.contains_key("PYTHONPATH"));
        for excluded in ["LD_PRELOAD", "API_TOKEN", "HTTPS_PROXY"] {
            assert!(!values.contains_key(excluded));
        }
        assert!(parse_environment(&vec![b'x'; 256 * 1024 + 1]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_capture_preserves_sample_output_and_reports_cli_failures() {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'sample'; sleep 10"]);
        let started = Instant::now();
        let output = capture(command, Duration::from_millis(100), true, &cancel).unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(output.contains("sample"));
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'broken package' >&2; exit 7"]);
        let error = capture(command, Duration::from_secs(1), true, &cancel).unwrap_err();
        assert!(error.to_string().contains("broken package"));
    }

    #[cfg(unix)]
    #[test]
    fn output_limit_and_cancel_stop_the_whole_query_group() {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "while :; do printf '0123456789012345678901234567890123456789'; done",
        ]);
        let output = capture(command, Duration::from_secs(2), true, &cancel).unwrap();
        assert!(output.len() < 257 * 1024);
        assert!(output.contains("Output truncated"));
        let worker_cancel = cancel.clone();
        let toggler = thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            worker_cancel.store(true, Ordering::Relaxed);
        });
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & wait"]);
        let started = Instant::now();
        assert!(capture(command, Duration::from_secs(5), true, &cancel).is_err());
        toggler.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
