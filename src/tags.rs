//! Local workload hints from sampled evidence. No helper commands or environment reads.
mod catalog;
use crate::model::{Process, Snapshot};
pub use catalog::{Capability, supports};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    #[default]
    Observed,
    Inferred,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tag {
    pub key: String,
    pub label: String,
    pub confidence: Confidence,
    pub evidence: Vec<String>,
}
impl Tag {
    pub fn display(&self) -> String {
        format!(
            "{}{}",
            self.label,
            if self.confidence == Confidence::Inferred {
                "?"
            } else {
                ""
            }
        )
    }
}
fn push(tags: &mut Vec<Tag>, key: &str, label: &str, confidence: Confidence, evidence: String) {
    if let Some(tag) = tags.iter_mut().find(|t| t.key == key) {
        if confidence == Confidence::Observed {
            tag.confidence = Confidence::Observed;
        }
        if !tag.evidence.contains(&evidence) && tag.evidence.len() < 4 {
            tag.evidence.push(evidence);
        }
    } else {
        tags.push(Tag {
            key: key.into(),
            label: label.into(),
            confidence,
            evidence: vec![evidence],
        });
    }
}
pub fn merge(target: &mut Vec<Tag>, source: &[Tag], relation: &str) {
    for tag in source {
        for evidence in &tag.evidence {
            push(
                target,
                &tag.key,
                &tag.label,
                tag.confidence,
                format!("{relation}: {evidence}"),
            );
        }
    }
}
pub fn text(tags: &[Tag]) -> String {
    tags.iter().map(Tag::display).collect::<Vec<_>>().join(" ")
}
pub fn matches(tags: &[Tag], query: &str) -> bool {
    let query = query.trim_end_matches('?').to_lowercase();
    if query.is_empty() {
        return false;
    }
    tags.iter()
        .any(|t| t.key.contains(&query) || t.label.to_lowercase().contains(&query))
}
pub fn explain(tags: &[Tag]) -> String {
    let mut body = String::from(
        "Labels describe sampled programs and resources. A ? marks an inferred purpose, not confirmed activity. No matching label means unrecognized, not inactive.\n",
    );
    for tag in tags {
        body.push_str(&format!(
            "\n{} · {}\n{}\n",
            tag.display(),
            if tag.confidence == Confidence::Inferred {
                "purpose hint"
            } else {
                "sampled evidence"
            },
            tag.evidence.join("\n")
        ));
    }
    body
}

pub fn query_matches(tags: &[Tag], query: &str, plain: &str) -> bool {
    let plain = format!("{} {}", plain.to_lowercase(), text(tags).to_lowercase());
    query.split_whitespace().all(|word| {
        let (negated, word) = word.strip_prefix('!').map_or((false, word), |v| (true, v));
        let matched = if let Some(value) = word.strip_prefix("tag:") {
            matches(tags, value)
        } else {
            plain.contains(&word.to_lowercase())
        };
        matched != negated
    })
}

fn base(value: &str) -> String {
    value
        .trim_matches(['\'', '"'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .to_ascii_lowercase()
        .trim_end_matches(".exe")
        .into()
}
fn family(name: &str, source: &str) -> Vec<Tag> {
    let mut tags = vec![];
    catalog::recognize(name, source, &mut tags);
    tags
}

/// Command strings are joined argv and can be ambiguous. Only explicit entry points are used.
fn entry_point<'a>(name: &str, words: &'a [&'a str]) -> Option<&'a str> {
    let python = ["python", "pythonw", "pypy", "pypy3"].contains(&name)
        || name
            .strip_prefix("python")
            .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'));
    let runtime = python
        || [
            "node",
            "nodejs",
            "deno",
            "bun",
            "java",
            "dotnet",
            "bash",
            "sh",
            "zsh",
            "pwsh",
            "powershell",
            "ruby",
            "perl",
            "php",
        ]
        .contains(&name);
    if !runtime {
        return None;
    }
    let mut i = 1;
    while i < words.len() {
        let word = words[i];
        if ["-c", "-e", "--eval", "-command"].contains(&word) {
            return None;
        }
        if ["-m", "-jar"].contains(&word) {
            return words.get(i + 1).copied();
        }
        if [
            "-W",
            "-X",
            "-r",
            "--require",
            "--import",
            "--loader",
            "--class-path",
            "-cp",
        ]
        .contains(&word)
        {
            i += 2;
            continue;
        }
        if !word.starts_with('-') {
            return Some(word);
        }
        i += 1;
    }
    None
}

pub fn process(p: &Process) -> Vec<Tag> {
    let name = base(&p.name);
    let words: Vec<_> = p.command.split_whitespace().take(128).collect();
    let mut tags = family(&name, "Process executable");
    if name.len() == 15
        && let Some(executable) = words.first().map(|word| base(word))
        && executable.starts_with(&name)
        && executable.len() > name.len()
    {
        catalog::recognize(
            &executable,
            "Command executable (matches truncated process name)",
            &mut tags,
        );
    }
    if let Some(entry) = entry_point(&name, &words) {
        let target = base(entry);
        let stem = target
            .trim_end_matches(".py")
            .trim_end_matches(".js")
            .trim_end_matches(".sh");
        catalog::recognize(stem, "Runtime entry point", &mut tags);
        if ["train", "training", "finetune", "fine_tune", "fine-tune"].contains(&stem) {
            push(
                &mut tags,
                "training",
                "Training",
                Confidence::Inferred,
                format!("Runtime entry point: {entry}; application progress is not sampled"),
            );
        }
        if ["pytest", "unittest", "nose2"].contains(&stem)
            || name == "node" && words.contains(&"--test")
        {
            push(
                &mut tags,
                "testing",
                "Tests",
                Confidence::Observed,
                format!("Test runner entry point: {entry}"),
            );
        }
        if stem == "http.server" || stem == "uvicorn" || stem == "gunicorn" {
            push(
                &mut tags,
                "web",
                "Web",
                Confidence::Observed,
                format!("Web runtime entry point: {entry}"),
            );
        }
        if ["vllm.entrypoints.openai.api_server", "sglang.launch_server"].contains(&stem) {
            push(
                &mut tags,
                "model-server",
                "Model server",
                Confidence::Inferred,
                format!("Model-serving entry point: {entry}; request handling is not sampled"),
            );
        }
        if stem == "torch.distributed.run" || stem == "accelerate.commands.launch" {
            push(
                &mut tags,
                "distributed-compute",
                "Distributed",
                Confidence::Observed,
                format!("Distributed launcher: {entry}; training vs inference is unknown"),
            );
        }
    }
    command_roles(&name, &words, &mut tags);
    let entry = entry_point(&name, &words).or_else(|| words.first().copied());
    if words.contains(&"--ros-args")
        || words
            .iter()
            .any(|v| v.starts_with("__name:=") || v.starts_with("__master:="))
        || entry.is_some_and(|v| v.starts_with("/opt/ros/") && v.contains("/lib/"))
    {
        push(&mut tags, "ros", "ROS", Confidence::Observed,
            "ROS entry point or explicit ROS command arguments; graph membership is checked on request".into());
    }
    if words.contains(&"--ros-args") {
        push(
            &mut tags,
            "robotics",
            "Robotics",
            Confidence::Observed,
            "Explicit --ros-args command argument".into(),
        );
    }
    if p.gpu_percent.is_some_and(|v| v > 0) {
        push(
            &mut tags,
            "gpu-active",
            "GPU active",
            Confidence::Observed,
            format!("Sampled GPU utilization {}%", p.gpu_percent.unwrap()),
        );
    } else if p.gpu_memory.is_some_and(|v| v > 0) || !p.gpu_ids.is_empty() {
        push(
            &mut tags,
            "gpu-memory",
            "GPU memory",
            Confidence::Observed,
            format!(
                "GPU devices {:?}; sampled allocation {:?} bytes; activity unknown",
                p.gpu_ids, p.gpu_memory
            ),
        );
    }
    if p.cpu >= 100. {
        push(
            &mut tags,
            "cpu-cores",
            "CPU >=1 core",
            Confidence::Observed,
            format!("Sampled CPU {:.1}% (100% = one core)", p.cpu),
        );
    }
    if p.read_per_sec + p.write_per_sec >= 10. * 1024. * 1024. {
        push(
            &mut tags,
            "disk-io",
            "Disk IO",
            Confidence::Observed,
            format!(
                "Sampled disk read {:.0} B/s + write {:.0} B/s",
                p.read_per_sec, p.write_per_sec
            ),
        );
    }
    if p.memory >= 1024 * 1024 * 1024 {
        push(
            &mut tags,
            "memory-heavy",
            "RAM >=1 GiB",
            Confidence::Observed,
            format!(
                "Sampled resident memory {} bytes; this is a size threshold, not a leak diagnosis",
                p.memory
            ),
        );
    }
    for (state, key, label) in [
        ("Zombie", "zombie", "Zombie"),
        ("Stop", "stopped", "Stopped"),
        ("Stopped", "stopped", "Stopped"),
    ] {
        if p.state.eq_ignore_ascii_case(state) {
            push(
                &mut tags,
                key,
                label,
                Confidence::Observed,
                format!("Sampled process state: {}", p.state),
            );
        }
    }
    tags.sort_by_key(|t| match t.key.as_str() {
        "training" | "model-server" | "testing" | "build" => 0,
        "distributed-compute" | "zenoh" => 1,
        "gpu-active" | "gpu-memory" => 2,
        _ => 3,
    });
    tags
}

/// Only explicit tool subcommands establish an activity. `run app.py` is not installation.
fn command_roles(name: &str, words: &[&str], tags: &mut Vec<Tag>) {
    let mut position = 1;
    if name == "cargo" && words.get(position).is_some_and(|w| w.starts_with('+')) {
        position += 1;
    }
    let command = words.get(position).copied().unwrap_or("");
    if ["npm", "pnpm", "yarn", "bun"].contains(&name) && command == "run" {
        let role = match words.get(position + 1).copied() {
            Some("build") => Some(("build", "Build")),
            Some("test") => Some(("testing", "Tests")),
            Some("lint" | "check") => Some(("lint", "Code check")),
            _ => None,
        };
        if let Some((key, label)) = role {
            push(
                tags,
                key,
                label,
                Confidence::Inferred,
                format!(
                    "Project-defined script: {name} run {}; script contents are not inspected",
                    words[position + 1]
                ),
            );
        }
    }
    let role = match (name, command) {
        ("cargo", "test" | "nextest")
        | ("go", "test")
        | ("dotnet", "test")
        | ("npm" | "pnpm" | "yarn" | "bun", "test") => Some(("testing", "Tests")),
        ("cargo", "build" | "check" | "clippy" | "rustc")
        | ("go", "build" | "install")
        | ("dotnet", "build" | "publish")
        | ("npm" | "pnpm" | "yarn" | "bun", "build")
        | ("uv", "build")
        | ("docker" | "podman" | "buildctl", "build") => Some(("build", "Build")),
        ("cargo", "install" | "fetch" | "update")
        | ("npm" | "pnpm" | "yarn" | "bun", "install" | "add" | "update" | "remove" | "ci")
        | ("dotnet", "restore")
        | ("uv", "sync" | "add" | "remove" | "pip" | "tool") => Some(("packages", "Packages")),
        ("npm" | "pnpm" | "yarn", "lint") | ("cargo", "fmt") => Some(("lint", "Code check")),
        ("vllm", "serve") | ("ollama", "serve") => Some(("model-server", "Model server")),
        ("jupyter", "lab" | "notebook" | "server") => Some(("notebook", "Notebook")),
        ("ros2", "bag") | ("rosbag", "record" | "play") => Some(("robotics", "Robotics")),
        _ => None,
    };
    if let Some((key, label)) = role {
        push(
            tags,
            key,
            label,
            Confidence::Observed,
            format!("Explicit command: {name} {command}"),
        );
    }
    if ["node", "nodejs"].contains(&name)
        && words
            .iter()
            .skip(1)
            .take_while(|w| w.starts_with('-'))
            .any(|w| *w == "--test")
    {
        push(
            tags,
            "testing",
            "Tests",
            Confidence::Observed,
            "Explicit node --test runner flag".into(),
        );
    }
}

/// Recompute after samples are merged; cached tags never survive a PID/resource change.
pub fn annotate(snapshot: &mut Snapshot) -> Vec<crate::applications::Application> {
    for p in &mut snapshot.processes {
        p.tags = process(p);
    }
    for s in &mut snapshot.inventory.services {
        s.tags = family(
            s.name
                .trim_end_matches(".service")
                .split('@')
                .next()
                .unwrap_or(""),
            "Service unit name",
        );
        for tag in &mut s.tags {
            tag.confidence = Confidence::Inferred;
        }
        if s.active == "failed" {
            push(
                &mut s.tags,
                "failed",
                "Failed",
                Confidence::Observed,
                format!("Service manager state: {} / {}", s.active, s.sub),
            );
        }
    }
    for c in &mut snapshot.inventory.containers {
        c.tags.clear();
        let image = c
            .image
            .rsplit('/')
            .next()
            .unwrap_or(&c.image)
            .split([':', '@'])
            .next()
            .unwrap_or("");
        for mut tag in family(image, "Container image basename") {
            tag.confidence = Confidence::Inferred;
            c.tags.push(tag);
        }
        if c.state == "restarting" {
            push(
                &mut c.tags,
                "restarting",
                "Restarting",
                Confidence::Observed,
                format!("Container runtime state: {}", c.state),
            );
        }
        if c.status.to_ascii_lowercase().contains("unhealthy") {
            push(
                &mut c.tags,
                "unhealthy",
                "Unhealthy",
                Confidence::Observed,
                format!("Container runtime health: {}", c.status),
            );
        }
    }
    // Ownership comes from the same passive index used by related-resource navigation.
    let apps = crate::applications::index(snapshot);
    let process_tags: HashMap<_, _> = snapshot
        .processes
        .iter()
        .filter(|p| !p.thread)
        .map(|p| ((p.pid, p.start_time), &p.tags))
        .collect();
    for app in &apps {
        let mut tags = vec![];
        for identity in &app.members {
            if let Some(source) = process_tags.get(identity) {
                merge(&mut tags, source, &format!("Member PID {}", identity.0));
            }
        }
        if let Some(i) = app.service {
            merge(
                &mut snapshot.inventory.services[i].tags,
                &tags,
                "Observed service members",
            );
        }
        if let Some(i) = app.container {
            merge(
                &mut snapshot.inventory.containers[i].tags,
                &tags,
                "Matched container cgroup members",
            );
        }
    }
    let owners: HashMap<_, _> = snapshot
        .processes
        .iter()
        .filter(|p| !p.thread)
        .map(|p| (p.pid, (p.start_time, &p.tags)))
        .collect();
    for connection in &mut snapshot.connections.connections {
        connection.tags.clear();
        for pid in &connection.pids {
            if let Some((start, tags)) = owners.get(pid)
                && (snapshot.connections.owners_sampled_at == 0
                    || *start <= snapshot.connections.owners_sampled_at)
            {
                // A resource tag belongs to the process as a whole, not to each socket.
                let roles: Vec<_> = tags
                    .iter()
                    .filter(|t| !is_resource(&t.key))
                    .cloned()
                    .collect();
                merge(
                    &mut connection.tags,
                    &roles,
                    &format!("Sampled owner PID {pid}"),
                );
            }
        }
        if connection.state == "LISTEN" {
            push(
                &mut connection.tags,
                "listener",
                "Listener",
                Confidence::Observed,
                format!("TCP socket state LISTEN at {}", connection.local),
            );
        }
        if ["SYN_SENT", "SYN_RECV"].contains(&connection.state.as_str()) {
            push(
                &mut connection.tags,
                "connecting",
                "Connecting",
                Confidence::Observed,
                format!(
                    "Sampled TCP state: {} (a single sample does not establish failure)",
                    connection.state
                ),
            );
        }
        if connection.receive_queue.unwrap_or(0) + connection.send_queue.unwrap_or(0) >= 64 * 1024 {
            push(
                &mut connection.tags,
                "socket-queue",
                "Socket queue",
                Confidence::Observed,
                format!(
                    "Sampled receive queue {:?} bytes / send queue {:?} bytes; queued data alone does not establish a fault",
                    connection.receive_queue, connection.send_queue
                ),
            );
        }
        if connection.rx_per_sec.unwrap_or(0.) + connection.tx_per_sec.unwrap_or(0.)
            >= 1024. * 1024.
        {
            push(
                &mut connection.tags,
                "network-io",
                "Network IO",
                Confidence::Observed,
                format!(
                    "Socket payload RX {:?} B/s / TX {:?} B/s",
                    connection.rx_per_sec, connection.tx_per_sec
                ),
            );
        }
    }
    apps
}
fn is_resource(key: &str) -> bool {
    [
        "gpu-active",
        "gpu-memory",
        "cpu-cores",
        "disk-io",
        "memory-heavy",
        "zombie",
        "stopped",
    ]
    .contains(&key)
}

pub fn summary(snapshot: &Snapshot) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::<String, usize>::new();
    for p in snapshot.processes.iter().filter(|p| !p.thread) {
        for tag in &p.tags {
            *counts.entry(tag.display()).or_default() += 1;
        }
    }
    counts.into_iter().collect()
}

pub(crate) fn check_identity(p: &Process) -> anyhow::Result<()> {
    let id = sysinfo::Pid::from_u32(p.pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[id]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    anyhow::ensure!(
        system
            .process(id)
            .is_some_and(|live| live.start_time() == p.start_time),
        "Process exited or PID identity changed"
    );
    Ok(())
}

/// Explicit, bounded resource inspection. No process control or environment access.
pub fn inspect_resources(p: &Process, memory: bool) -> anyhow::Result<String> {
    check_identity(p)?;
    let mut body = format!(
        "PID {} · {}\nSampled CPU {:.1}% · resident {} bytes\n",
        p.pid, p.name, p.cpu, p.memory
    );
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        let files: &[&str] = if memory {
            &[
                "status",
                "smaps_rollup",
                "statm",
                "limits",
                "oom_score",
                "oom_score_adj",
                "cgroup",
            ]
        } else {
            &[
                "status",
                "schedstat",
                "sched",
                "autogroup",
                "wchan",
                "cgroup",
            ]
        };
        for file in files {
            let result = (|| -> std::io::Result<String> {
                let mut text = String::new();
                std::fs::File::open(format!("/proc/{}/{file}", p.pid))?
                    .take(64 * 1024 + 1)
                    .read_to_string(&mut text)?;
                if text.len() > 64 * 1024 {
                    text.truncate(text.floor_char_boundary(64 * 1024));
                    text.push_str("\nRead limit reached; incomplete.");
                }
                Ok(text)
            })();
            body.push_str(&format!(
                "\n{file}\n{}\n",
                result.unwrap_or_else(|e| e.to_string())
            ));
        }
        body.push_str("\nHost-visible native fields; permissions and process namespaces may limit visibility.\n");
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = memory;
        body.push_str("Native resource inspection is Linux-only.\n");
    }
    check_identity(p)?;
    Ok(body)
}

/// Explicit action with identity checks around bounded native reads.
pub fn inspect(p: &Process, environment: bool) -> anyhow::Result<String> {
    check_identity(p)?;
    #[cfg(target_os = "linux")]
    let body = {
        use std::{
            fs,
            io::Read,
            time::{Duration, Instant},
        };
        if environment {
            let mut bytes = vec![];
            fs::File::open(format!("/proc/{}/environ", p.pid))?
                .take(256 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            let truncated = bytes.len() > 256 * 1024;
            bytes.truncate(256 * 1024);
            let values: Vec<_> = String::from_utf8_lossy(&bytes)
                .split('\0')
                .filter(|value| {
                    let key = value.split_once('=').map_or(*value, |(key, _)| key);
                    ["ROS_", "RMW_", "ZENOH_"]
                        .iter()
                        .any(|prefix| key.starts_with(prefix))
                        || [
                            "CYCLONEDDS_URI",
                            "FASTRTPS_DEFAULT_PROFILES_FILE",
                            "FASTDDS_DEFAULT_PROFILES_FILE",
                        ]
                        .contains(&key)
                })
                .map(str::to_string)
                .collect();
            format!(
                "Communication configuration for PID {}\n{}\n{}\nValues describe process configuration, not a confirmed graph, active transport or QoS compatibility.",
                p.pid,
                if values.is_empty() {
                    "No matching variables were visible (defaults may apply).".into()
                } else {
                    values.join("\n")
                },
                if truncated {
                    "Environment read reached the 256 KiB limit; incomplete."
                } else {
                    ""
                }
            )
        } else {
            let began = Instant::now();
            let mut files = vec![];
            for entry in fs::read_dir(format!("/proc/{}/fd", p.pid))?
                .flatten()
                .take(4096)
            {
                if began.elapsed() > Duration::from_millis(500) {
                    files.push("500 ms inspection budget reached; incomplete".into());
                    break;
                }
                files.push(format!(
                    "{} → {}",
                    entry.file_name().to_string_lossy(),
                    fs::read_link(entry.path())
                        .map(|v| v.display().to_string())
                        .unwrap_or_else(|e| e.to_string())
                ));
            }
            files.sort();
            format!(
                "Up to 4096 visible file descriptors (500 ms budget):\n{}",
                files.join("\n")
            )
        }
    };
    #[cfg(not(target_os = "linux"))]
    let body = {
        let _ = environment;
        "This label inspection is currently Linux-only; use the platform process tools for available fields.".into()
    };
    check_identity(p)?;
    Ok(body)
}
