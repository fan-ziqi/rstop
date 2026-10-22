//! A passive application index. Classification is a hint; ownership needs sampled evidence.
use crate::model::{Process, Snapshot};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub category: String,
    pub state: String,
    pub user: String,
    pub cpu: Option<f64>,
    /// Sum of resident memory, which can count shared pages more than once.
    pub rss: Option<u64>,
    pub read_per_sec: Option<f64>,
    pub write_per_sec: Option<f64>,
    pub members: Vec<(u32, u64)>,
    pub sockets: Vec<usize>,
    pub service: Option<usize>,
    pub container: Option<usize>,
    pub evidence: Vec<String>,
    #[serde(skip)]
    search: String,
}

fn basename(text: &str) -> &str {
    text.trim_matches(['\'', '"'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(text)
        .trim_end_matches(".exe")
}

/// Match executable identities, never arbitrary substrings in arguments or port numbers.
pub fn category(name: &str) -> &'static str {
    let name = basename(name).to_ascii_lowercase();
    let name = name.trim_end_matches(".exe");
    if [
        "nginx", "apache2", "httpd", "caddy", "traefik", "envoy", "haproxy", "gunicorn", "uvicorn",
        "uwsgi",
    ]
    .contains(&name)
    {
        "Web / proxy"
    } else if [
        "postgres",
        "postmaster",
        "mysqld",
        "mariadbd",
        "mongod",
        "cockroach",
        "clickhouse-server",
        "sqlservr",
        "influxd",
    ]
    .contains(&name)
    {
        "Database"
    } else if ["redis-server", "valkey-server", "memcached", "keydb-server"].contains(&name) {
        "Cache"
    } else if [
        "ros2",
        "roscore",
        "rosmaster",
        "roslaunch",
        "rosout",
        "rviz",
        "rviz2",
        "gzserver",
        "gzclient",
        "gazebo",
        "component_container",
        "component_container_mt",
    ]
    .contains(&name)
    {
        "Robotics / simulation"
    } else if [
        "zenohd",
        "zenoh-bridge-ros2dds",
        "zenoh-bridge-dds",
        "fastdds",
        "ddsrouter",
        "discovery-server",
        "mosquitto",
        "nats-server",
        "rabbitmq-server",
        "kafka-server-start.sh",
        "redpanda",
    ]
    .contains(&name)
    {
        "Messaging / middleware"
    } else if [
        "ollama",
        "vllm",
        "torchrun",
        "llama-server",
        "tritonserver",
        "text-generation-launcher",
    ]
    .contains(&name)
    {
        "AI / compute"
    } else if [
        "prometheus",
        "grafana",
        "grafana-server",
        "loki",
        "otelcol",
        "otelcol-contrib",
        "node_exporter",
        "vector",
        "fluent-bit",
        "telegraf",
    ]
    .contains(&name)
    {
        "Observability"
    } else if [
        "dockerd",
        "containerd",
        "podman",
        "conmon",
        "runc",
        "kubelet",
        "k3s",
        "k3s-server",
        "kube-apiserver",
    ]
    .contains(&name)
    {
        "Containers / orchestration"
    } else if name.starts_with("qemu-system-")
        || [
            "virtualboxvm",
            "vboxheadless",
            "vmware-vmx",
            "libvirtd",
            "virtqemud",
            "wsl",
            "vmmem",
        ]
        .contains(&name)
    {
        "Virtualization"
    } else if [
        "ssh",
        "sshd",
        "tailscaled",
        "openvpn",
        "wireguard-go",
        "wg-quick",
        "mihomo",
        "clash",
        "sing-box",
        "syncthing",
        "rsync",
        "networkmanager",
        "systemd-resolved",
        "dnsmasq",
        "named",
        "unbound",
    ]
    .contains(&name)
    {
        "Network / sync"
    } else if [
        "chrome",
        "chromium",
        "chromium-browser",
        "firefox",
        "msedge",
        "safari",
        "brave",
        "brave-browser",
    ]
    .contains(&name)
    {
        "Browser"
    } else if [
        "code",
        "codium",
        "vim",
        "nvim",
        "emacs",
        "idea",
        "pycharm",
        "clion",
        "zed",
        "sublime_text",
    ]
    .contains(&name)
    {
        "Editor / IDE"
    } else if [
        "gcc",
        "g++",
        "cc",
        "cc1",
        "cc1plus",
        "clang",
        "clang++",
        "rustc",
        "cargo",
        "cmake",
        "ninja",
        "make",
        "bazel",
        "ld",
        "lld",
        "rust-analyzer",
    ]
    .contains(&name)
    {
        "Build / development"
    } else if [
        "ffmpeg",
        "ffplay",
        "vlc",
        "mpv",
        "obs",
        "obs64",
        "blender",
        "pipewire",
        "pulseaudio",
        "ardour",
    ]
    .contains(&name)
    {
        "Media / graphics"
    } else if is_runtime(name) {
        "Language runtime"
    } else {
        "Program"
    }
}

fn is_runtime(name: &str) -> bool {
    [
        "node",
        "nodejs",
        "java",
        "javaw",
        "dotnet",
        "ruby",
        "php",
        "julia",
        "deno",
        "bun",
        "python",
        "pythonw",
        "pypy",
        "pypy3",
        "bash",
        "sh",
        "zsh",
        "fish",
        "dash",
        "pwsh",
        "powershell",
        "cmd",
    ]
    .contains(&name)
        || name
            .strip_prefix("python")
            .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

fn identity(p: &Process) -> (String, String, String) {
    let executable = p.command.split_whitespace().next().unwrap_or(&p.name);
    let program = basename(executable);
    // A rewritten process title (nginx workers, postgres backends) is not an executable path.
    let program = if program.is_empty() || program.ends_with(':') {
        &p.name
    } else {
        program
    };
    let mut category = category(program).to_string();
    let words: Vec<_> = p.command.split_whitespace().collect();
    let process_program = basename(&p.name).to_ascii_lowercase();
    let process_program = process_program.trim_end_matches(".exe");
    let runtime = is_runtime(&program.to_ascii_lowercase()) || is_runtime(process_program);
    let program = if is_runtime(process_program) {
        p.name.as_str()
    } else {
        program
    };
    let name = if runtime {
        let target = words.iter().skip(1).find(|w| !w.starts_with('-'));
        // Keep separate invocations of a shared interpreter separate, including -c / -m / -jar.
        target.map_or_else(
            || program.to_string(),
            |target| format!("{program} · {target}"),
        )
    } else {
        p.name.clone()
    };
    if category == "Program" {
        category = self::category(&p.name).into();
    }
    if words.contains(&"--ros-args") || executable.starts_with("/opt/ros/") {
        category = "Robotics / simulation".into();
    }
    let key = if runtime {
        p.command.clone()
    } else {
        format!("{executable}:{}", p.name)
    };
    (format!("program:{}:{key}", p.user), name, category)
}

fn add_search(app: &mut Application, text: &str) {
    app.search.push(' ');
    app.search.push_str(&text.to_lowercase());
}

fn cgroup_contains(cgroup: &str, id: &str) -> bool {
    id.len() >= 12
        && cgroup.split(['/', ':', '\n']).any(|part| {
            part == id
                || part
                    .strip_prefix("docker-")
                    .or_else(|| part.strip_prefix("libpod-"))
                    .and_then(|v| v.strip_suffix(".scope"))
                    == Some(id)
        })
}

pub fn index(snapshot: &Snapshot) -> Vec<Application> {
    let mut groups = BTreeMap::<String, Application>::new();
    let processes: BTreeMap<_, _> = snapshot
        .processes
        .iter()
        .filter(|p| !p.thread)
        .map(|p| (p.pid, p))
        .collect();
    let mut service_pids = HashMap::<u32, Vec<usize>>::new();
    for (i, service) in snapshot.inventory.services.iter().enumerate() {
        if let Some(pid) = service.pid.filter(|pid| *pid > 1) {
            service_pids.entry(pid).or_default().push(i);
        }
    }
    for (i, s) in snapshot.inventory.services.iter().enumerate() {
        let id = format!("service:{}:{}", s.scope, s.name);
        let mut app = Application {
            id: id.clone(),
            name: s.name.clone(),
            kind: "Service".into(),
            state: s.active.clone(),
            category: category(s.name.trim_end_matches(".service")).into(),
            service: Some(i),
            evidence: vec![
                "Service manager inventory; processes are attached only with ownership evidence"
                    .into(),
            ],
            ..Default::default()
        };
        add_search(
            &mut app,
            &format!("{} {} {} {}", s.name, s.scope, s.active, s.description),
        );
        groups.insert(id, app);
    }
    for (i, c) in snapshot.inventory.containers.iter().enumerate() {
        let id = format!("container:{}:{}", c.runtime, c.id);
        let mut app = Application {
            id: id.clone(),
            name: c.name.clone(),
            kind: "Container".into(),
            state: c.state.clone(),
            category: "Container workload".into(),
            container: Some(i),
            evidence: vec![
                "Container runtime inventory; host processes require a matching sampled cgroup"
                    .into(),
            ],
            ..Default::default()
        };
        add_search(
            &mut app,
            &format!(
                "{} {} {} {} {} {}",
                c.id, c.name, c.runtime, c.image, c.command, c.status
            ),
        );
        groups.insert(id, app);
    }
    let mut pid_groups = HashMap::new();
    for p in processes.values() {
        let cgroup = p
            .extra
            .get("cgroup")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                snapshot
                    .inventory
                    .extra
                    .get("Application ownership")?
                    .get("cgroups")?
                    .get(format!("{}:{}", p.pid, p.start_time))?
                    .as_str()
            })
            .unwrap_or("");
        let container = snapshot
            .inventory
            .containers
            .iter()
            .find(|c| cgroup_contains(cgroup, &c.id));
        // Only follow observed ancestors; cycles and missing/reused parents stop attribution.
        let mut service = None;
        let mut ancestor = *p;
        let mut visited = HashSet::new();
        for _ in 0..64 {
            if !visited.insert(ancestor.pid) {
                break;
            }
            if let Some(indices) = service_pids.get(&ancestor.pid) {
                let manager = [
                    "systemd",
                    "launchd",
                    "services.exe",
                    "svchost.exe",
                    "containerd",
                    "dockerd",
                    "sshd",
                    "dbus-daemon",
                    "dbus-broker",
                    "gnome-session-binary",
                    "gdm-session-worker",
                ]
                .contains(&ancestor.name.as_str());
                if indices.len() == 1
                    && (ancestor.pid == p.pid || !manager)
                    // A service can launch independent applications (desktop sessions, SSH,
                    // schedulers). Keep them discoverable under their own executable/job.
                    && (ancestor.pid == p.pid || (
                        ancestor.name == p.name
                        && (!is_runtime(basename(&p.name)) || ancestor.command == p.command)
                    ))
                    && (snapshot.inventory.updated_at == 0
                        || ancestor.start_time <= snapshot.inventory.updated_at)
                {
                    service = Some(indices[0]);
                }
                // Shared service hosts and session/container managers do not own every descendant application.
                break;
            }
            let Some(parent) = ancestor.parent.and_then(|pid| processes.get(&pid)) else {
                break;
            };
            if parent.start_time > ancestor.start_time {
                break;
            }
            ancestor = parent;
        }
        let (generic_id, name, category) = identity(p);
        let (id, evidence) = if let Some(c) = container {
            (
                format!("container:{}:{}", c.runtime, c.id),
                "Container ID in sampled cgroup",
            )
        } else if let Some(i) = service {
            let s = &snapshot.inventory.services[i];
            (
                format!("service:{}:{}", s.scope, s.name),
                "Service MainPID / sampled process ancestry",
            )
        } else {
            (
                generic_id,
                "Executable/title + user; runtime invocations use the complete command",
            )
        };
        let app = groups.entry(id.clone()).or_insert_with(|| Application {
            id: id.clone(),
            name,
            kind: "Program".into(),
            state: "running".into(),
            category: category.clone(),
            ..Default::default()
        });
        if app.category == "Program" || app.category == "Container workload" {
            app.category = category;
        }
        if !app.evidence.iter().any(|v| v == evidence) {
            app.evidence.push(evidence.into());
        }
        if app.user.is_empty() {
            app.user = p.user.clone();
        } else if app.user != p.user {
            app.user = "multiple".into();
        }
        app.members.push((p.pid, p.start_time));
        *app.cpu.get_or_insert(0.) += p.cpu as f64;
        app.rss = Some(app.rss.unwrap_or(0).saturating_add(p.memory));
        *app.read_per_sec.get_or_insert(0.) += p.read_per_sec;
        *app.write_per_sec.get_or_insert(0.) += p.write_per_sec;
        add_search(
            app,
            &format!("{} {} {} {} {}", p.pid, p.name, p.user, p.command, p.state),
        );
        pid_groups.insert(p.pid, id);
    }
    for (i, c) in snapshot.connections.connections.iter().enumerate() {
        let ids: HashSet<_> = c
            .pids
            .iter()
            .filter(|pid| {
                processes.get(pid).is_some_and(|p| {
                    snapshot.connections.owners_sampled_at == 0
                        || p.start_time <= snapshot.connections.owners_sampled_at
                })
            })
            .filter_map(|pid| pid_groups.get(pid))
            .collect();
        for id in ids {
            let app = groups.get_mut(id).expect("indexed process group");
            app.sockets.push(i);
            add_search(
                app,
                &format!("{} {} {} {}", c.local, c.remote, c.protocol, c.state),
            );
        }
    }
    for app in groups.values_mut() {
        app.members.sort_unstable();
        if let Some(i) = app.container {
            let c = &snapshot.inventory.containers[i];
            app.cpu = c.cpu.or(app.cpu);
            app.rss = c.memory.or(app.rss);
        } else if let Some(i) = app.service {
            let s = &snapshot.inventory.services[i];
            // Service CPU is MainPID only; use summed sampled member CPU when available.
            app.cpu = app.cpu.or(s.cpu_percent.map(f64::from));
            app.rss = s.memory.or(app.rss);
        }
        let summary = format!("{} {} {} {}", app.name, app.kind, app.category, app.state);
        add_search(app, &summary);
    }
    let mut rows: Vec<_> = groups.into_values().collect();
    rows.sort_by(|a, b| {
        b.cpu
            .unwrap_or(-1.)
            .total_cmp(&a.cpu.unwrap_or(-1.))
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });
    rows
}

impl Application {
    pub fn matches(&self, query: &str) -> bool {
        query
            .split_whitespace()
            .all(|word| self.search.contains(&word.to_lowercase()))
    }
}

/// Explicit inspection only: never run programs, join middleware domains or read environments.
pub fn inspect_members(members: &[Process]) -> String {
    #[cfg(target_os = "linux")]
    {
        use std::{collections::BTreeSet, fs, io::Read};
        let mut body = String::from("\nLive runtime evidence (up to 8 sampled members)\n");
        for p in members.iter().take(8) {
            let inspect = || -> anyhow::Result<String> {
                let check = || -> anyhow::Result<()> {
                    let pid = sysinfo::Pid::from_u32(p.pid);
                    let mut system = sysinfo::System::new();
                    system.refresh_processes_specifics(
                        sysinfo::ProcessesToUpdate::Some(&[pid]),
                        true,
                        sysinfo::ProcessRefreshKind::nothing(),
                    );
                    anyhow::ensure!(
                        system
                            .process(pid)
                            .is_some_and(|live| live.start_time() == p.start_time),
                        "Process exited or PID identity changed"
                    );
                    Ok(())
                };
                check()?;
                let mut text = format!("\nPID {} · {}\n", p.pid, p.name);
                for (label, file) in [
                    ("Executable", "exe"),
                    ("Working directory", "cwd"),
                    ("Network namespace", "ns/net"),
                ] {
                    match fs::read_link(format!("/proc/{}/{file}", p.pid)) {
                        Ok(path) => text.push_str(&format!("{label}: {}\n", path.display())),
                        Err(error) => text.push_str(&format!("{label}: {error}\n")),
                    }
                }
                match fs::File::open(format!("/proc/{}/maps", p.pid)) {
                    Ok(file) => {
                        let mut data = Vec::new();
                        file.take(1024 * 1024 + 1).read_to_end(&mut data)?;
                        let truncated = data.len() > 1024 * 1024;
                        data.truncate(1024 * 1024);
                        let libraries: BTreeSet<_> = String::from_utf8_lossy(&data)
                            .lines()
                            .filter_map(|line| {
                                let path = line.split_whitespace().nth(5)?;
                                let name = basename(path);
                                (name.contains(".so") || name.ends_with(".dylib"))
                                    .then(|| name.to_string())
                            })
                            .collect();
                        text.push_str(&format!(
                            "Loaded shared objects ({} observed; up to 80 shown):\n{}\n",
                            libraries.len(),
                            libraries
                                .iter()
                                .take(80)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("\n")
                        ));
                        if truncated {
                            text.push_str("Memory-map read reached the 1 MiB limit; incomplete.\n");
                        }
                    }
                    Err(error) => {
                        text.push_str(&format!("Loaded components unavailable: {error}\n"))
                    }
                }
                check()?;
                Ok(text)
            };
            match inspect() {
                Ok(text) => body.push_str(&text),
                Err(error) => body.push_str(&format!("\nPID {}: {error}\n", p.pid)),
            }
        }
        body.push_str("Loaded libraries are component evidence, not proof of active traffic or application health. Environments are available only through the explicit process environment action.\n");
        body
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = members;
        "\nLoaded-component inspection is currently Linux-only. Use member processes for the available platform tools.\n".into()
    }
}

/// Low-frequency, bounded container attribution. Never runs in the host sampling loop.
pub fn discover_ownership(inventory: &mut crate::model::Inventory) {
    #[cfg(target_os = "linux")]
    {
        use std::{
            fs,
            time::{Duration, Instant},
        };
        if inventory.containers.is_empty() {
            return;
        }
        let began = Instant::now();
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        let mut cgroups = BTreeMap::new();
        let mut complete = true;
        let mut unreadable = 0;
        let mut processes: Vec<_> = system.processes().values().collect();
        processes.sort_by_key(|p| p.pid());
        for p in processes {
            if began.elapsed() >= Duration::from_millis(200) {
                complete = false;
                break;
            }
            match fs::read_to_string(format!("/proc/{}/cgroup", p.pid())) {
                Ok(text)
                    if inventory
                        .containers
                        .iter()
                        .any(|c| cgroup_contains(&text, &c.id)) =>
                {
                    cgroups.insert(format!("{}:{}", p.pid(), p.start_time()), text);
                }
                Err(_) => unreadable += 1,
                _ => {}
            }
        }
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        cgroups.retain(|identity, _| {
            identity
                .split_once(':')
                .and_then(|(pid, start)| {
                    Some((pid.parse::<usize>().ok()?, start.parse::<u64>().ok()?))
                })
                .is_some_and(|(pid, start)| {
                    system
                        .process(sysinfo::Pid::from(pid))
                        .is_some_and(|p| p.start_time() == start)
                })
        });
        inventory.extra.insert("Application ownership".into(), serde_json::json!({"cgroups":cgroups,"complete":complete,"unreadable_processes":unreadable}));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = inventory;
}
