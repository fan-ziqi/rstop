//! Slow, read-only integrations live on their own worker; never stall host sampling.
use crate::{
    collector::unix_time,
    model::{Container, Inventory, Listener, Service},
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Rates use successive complete discovery samples, never the fast UI refresh interval.
type ContainerCounters = (f64, [Option<u64>; 4]);
#[derive(Default)]
pub struct InventoryRates {
    previous: HashMap<(String, String), ContainerCounters>,
    began: Option<Instant>,
}
impl InventoryRates {
    pub fn apply(&mut self, inventory: &mut Inventory) {
        let began = *self.began.get_or_insert_with(Instant::now);
        self.apply_at(inventory, began.elapsed().as_secs_f64());
    }
    pub fn apply_at(&mut self, inventory: &mut Inventory, seconds: f64) {
        self.previous.retain(|(runtime, id), _| {
            inventory
                .containers
                .iter()
                .any(|c| &c.runtime == runtime && &c.id == id)
        });
        for container in &mut inventory.containers {
            let key = (container.runtime.clone(), container.id.clone());
            let counters = [
                container.network_rx,
                container.network_tx,
                container.block_read,
                container.block_write,
            ];
            let old = self.previous.insert(key, (seconds, counters));
            let rates: [Option<f64>; 4] = std::array::from_fn(|i| {
                let (at, previous) = old?;
                let elapsed = seconds - at;
                if !elapsed.is_finite() || elapsed <= 0. {
                    return None;
                }
                Some(counters[i]?.checked_sub(previous[i]?)? as f64 / elapsed)
            });
            container.network_rx_per_sec = rates[0];
            container.network_tx_per_sec = rates[1];
            container.block_read_per_sec = rates[2];
            container.block_write_per_sec = rates[3];
        }
    }
}

pub fn discover() -> Inventory {
    discover_with_config(&crate::config::Config::default())
}

pub fn discover_with_config(config: &crate::config::Config) -> Inventory {
    discover_selected(config, true, true, true)
}

pub fn discover_selected(
    config: &crate::config::Config,
    containers: bool,
    services: bool,
    extras: bool,
) -> Inventory {
    discover_selected_impl(config, containers, services, extras, true, &mut |_| {})
}

/// Publish initial container lists before statistics; platform GPU counters use their own worker.
pub fn discover_background(
    config: &crate::config::Config,
    containers: bool,
    services: bool,
    extras: bool,
    mut publish: impl FnMut(&Inventory),
) -> Inventory {
    discover_selected_impl(config, containers, services, extras, false, &mut publish)
}

fn discover_selected_impl(
    config: &crate::config::Config,
    containers: bool,
    services: bool,
    extras: bool,
    platform_gpu: bool,
    publish: &mut impl FnMut(&Inventory),
) -> Inventory {
    let mut inventory = Inventory {
        updated_at: unix_time(),
        ..Inventory::default()
    };
    for runtime in ["docker", "podman"] {
        if !containers {
            break;
        }
        if let Err(error) = local_runtime(runtime) {
            if !error.to_string().contains("not installed") {
                inventory.notes.push(format!("{runtime}: {error}"));
            }
            continue;
        }
        #[cfg(unix)]
        if runtime == "docker"
            && let Ok(data) = docker_endpoint().and_then(|endpoint| {
                crate::docker::discover_with_updates(&endpoint, |partial| {
                    let mut ready = inventory.clone();
                    ready.runtimes.push(runtime.into());
                    ready.containers.extend(partial.containers.clone());
                    publish(&ready);
                })
            })
        {
            inventory.runtimes.push(runtime.into());
            inventory.containers.extend(data.containers);
            inventory.notes.extend(data.notes);
            publish(&inventory);
            continue;
        }
        match command(
            runtime,
            &["ps", "-a", "--no-trunc", "--format", "{{json .}}"],
            Duration::from_secs(3),
        ) {
            Ok(output) => {
                inventory.runtimes.push(runtime.to_string());
                let mut containers = parse_containers(runtime, &output);
                let mut ready = inventory.clone();
                ready.containers.extend(containers.clone());
                publish(&ready);
                if containers
                    .iter()
                    .any(|c| c.state == "running" || c.state == "Up")
                {
                    match command(
                        runtime,
                        &["stats", "--no-stream", "--format", "{{json .}}"],
                        Duration::from_secs(5),
                    ) {
                        Ok(stats) => apply_container_stats(&mut containers, &stats),
                        Err(error) => inventory.notes.push(format!("{runtime} stats: {error}")),
                    }
                }
                inventory.containers.extend(containers);
            }
            Err(error) => {
                if !error.to_string().contains("not installed") {
                    inventory.notes.push(format!("{runtime}: {error}"));
                }
            }
        }
        publish(&inventory);
    }
    if services {
        discover_services(&mut inventory);
    }
    inventory.containers.sort_by(|a, b| a.name.cmp(&b.name));
    inventory.services.sort_by(|a, b| {
        (a.active != "failed")
            .cmp(&(b.active != "failed"))
            .then(a.name.cmp(&b.name))
            .then(a.scope.cmp(&b.scope))
    });
    if extras {
        inventory.listeners = listeners();
        inventory.extra = if platform_gpu {
            crate::advanced::discover(config)
        } else {
            crate::advanced::discover_without_platform_gpu(config)
        };
    }
    inventory.extra.insert(
        "Discovery state".into(),
        serde_json::json!({"containers":containers,"services":services,"extras":extras}),
    );
    if containers {
        crate::applications::discover_ownership(&mut inventory);
    }
    inventory
}

pub fn is_local_endpoint(endpoint: &str) -> bool {
    if endpoint.starts_with("unix://") || endpoint.starts_with("npipe:////./pipe/") {
        return true;
    }
    let Some(tcp) = endpoint.strip_prefix("tcp://") else {
        return false;
    };
    let authority = tcp.split('/').next().unwrap_or(tcp);
    let (host, port) = if let Some(ipv6) = authority.strip_prefix('[') {
        let Some((host, suffix)) = ipv6.split_once(']') else {
            return false;
        };
        (host, suffix.strip_prefix(':').unwrap_or(suffix))
    } else {
        authority.split_once(':').unwrap_or((authority, ""))
    };
    ["localhost", "127.0.0.1", "::1"].contains(&host)
        && (port.is_empty() || port.chars().all(|c| c.is_ascii_digit()))
}

fn local_runtime(runtime: &str) -> Result<()> {
    if runtime == "docker" {
        docker_endpoint()?;
    }
    #[cfg(target_os = "linux")]
    if runtime == "podman" { /* command_impl forces the local engine */ }
    #[cfg(not(target_os = "linux"))]
    if runtime == "podman" {
        podman_endpoint()?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn podman_endpoint() -> Result<(String, Option<String>)> {
    if let Ok(host) = std::env::var("CONTAINER_HOST") {
        anyhow::ensure!(
            is_local_podman_endpoint(&host),
            "Nonlocal Podman endpoint skipped"
        );
        return Ok((host, std::env::var("CONTAINER_SSHKEY").ok()));
    }
    let output = command(
        "podman",
        &["system", "connection", "list", "--format", "json"],
        Duration::from_secs(2),
    )?;
    let values: Vec<Value> = serde_json::from_str(&output)?;
    let name = std::env::var("CONTAINER_CONNECTION").ok();
    let connection = values
        .iter()
        .find(|v| {
            if let Some(name) = &name {
                v["Name"].as_str() == Some(name)
            } else {
                v["Default"].as_bool() == Some(true)
            }
        })
        .context("No default local Podman connection")?;
    let endpoint = field(connection, &["URI"]);
    anyhow::ensure!(
        is_local_podman_endpoint(&endpoint),
        "Nonlocal Podman connection skipped"
    );
    Ok((
        endpoint,
        connection["Identity"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(ToString::to_string),
    ))
}

#[cfg(not(target_os = "linux"))]
fn is_local_podman_endpoint(endpoint: &str) -> bool {
    if is_local_endpoint(endpoint) {
        return true;
    }
    let Some(ssh) = endpoint.strip_prefix("ssh://") else {
        return false;
    };
    let host = ssh.rsplit('@').next().unwrap_or(ssh);
    ["127.0.0.1:", "localhost:", "[::1]:"]
        .iter()
        .any(|prefix| host.starts_with(prefix))
}

fn docker_endpoint() -> Result<String> {
    let context = std::env::var("DOCKER_CONTEXT")
        .ok()
        .filter(|s| !s.is_empty());
    let configured_context = if context.is_none() {
        let root = std::env::var_os("DOCKER_CONFIG")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".docker"))
            });
        root.and_then(|p| std::fs::read(p.join("config.json")).ok())
            .and_then(|s| serde_json::from_slice::<Value>(&s).ok())
            .and_then(|v| v["currentContext"].as_str().map(str::to_owned))
    } else {
        None
    };
    let endpoint = if context.is_none()
        && let Ok(host) = std::env::var("DOCKER_HOST")
    {
        host
    } else if context
        .as_deref()
        .or(configured_context.as_deref())
        .is_none_or(|s| s.is_empty() || s == "default")
    {
        #[cfg(windows)]
        {
            "npipe:////./pipe/docker_engine".into()
        }
        #[cfg(not(windows))]
        {
            "unix:///var/run/docker.sock".into()
        }
    } else {
        let mut args = vec!["context", "inspect"];
        if let Some(context) = context.as_deref() {
            args.push(context);
        }
        args.extend(["--format", "{{json .Endpoints.docker.Host}}"]);
        let text = command("docker", &args, Duration::from_secs(2))?;
        serde_json::from_str::<String>(text.trim()).context("Cannot determine Docker endpoint")?
    };
    anyhow::ensure!(
        is_local_endpoint(&endpoint),
        "Nonlocal Docker context skipped (local monitoring only)"
    );
    Ok(endpoint)
}

/// No shell interpolation; stdout/stderr are drained concurrently to avoid pipe deadlocks.
pub fn command(program: &str, args: &[&str], timeout: Duration) -> Result<String> {
    command_impl(program, args, timeout, false, false)
}

pub fn command_with_stderr(program: &str, args: &[&str], timeout: Duration) -> Result<String> {
    command_impl(program, args, timeout, true, false)
}

pub fn command_partial(program: &str, args: &[&str], timeout: Duration) -> Result<String> {
    command_impl(program, args, timeout, true, true)
}

fn command_impl(
    program: &str,
    args: &[&str],
    timeout: Duration,
    include_stderr: bool,
    allow_partial: bool,
) -> Result<String> {
    let mut command = Command::new(program);
    if program == "kubectl" {
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            command.env_remove(key);
        }
        command.env("NO_PROXY", "localhost,127.0.0.1,::1");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    if program == "docker" && args.first() != Some(&"context") {
        let endpoint = docker_endpoint()?;
        command
            .args(["--host", &endpoint])
            .env_remove("DOCKER_HOST")
            .env_remove("DOCKER_CONTEXT");
    }
    #[cfg(target_os = "linux")]
    if program == "podman" {
        command
            .arg("--remote=false")
            .env_remove("CONTAINER_HOST")
            .env_remove("CONTAINER_CONNECTION");
    }
    #[cfg(not(target_os = "linux"))]
    if program == "podman" && args.first() != Some(&"system") {
        let (endpoint, identity) = podman_endpoint()?;
        command
            .args(["--url", &endpoint])
            .env_remove("CONTAINER_HOST")
            .env_remove("CONTAINER_CONNECTION");
        if let Some(identity) = identity {
            command.args(["--identity", &identity]);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut process = command
        .args(args)
        .env("LC_ALL", "C")
        .env("SYSTEMD_COLORS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("{program} not installed or cannot be started"))?;
    #[cfg(windows)]
    let query_job = match crate::windows_inspect::QueryJob::attach(&process) {
        Ok(job) => job,
        Err(error) => {
            let _ = process.kill();
            let _ = process.wait();
            return Err(error);
        }
    };
    let stdout = process.stdout.take().expect("piped stdout");
    let stderr = process.stderr.take().expect("piped stderr");
    let out = thread::spawn(move || {
        let mut data = vec![];
        stdout
            .take(16 * 1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let err = thread::spawn(move || {
        let mut data = vec![];
        stderr
            .take(1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let started = Instant::now();
    let result = loop {
        if let Some(status) = process.try_wait()? {
            break Ok(status);
        }
        if started.elapsed() >= timeout || crate::STOP.load(std::sync::atomic::Ordering::Relaxed) {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(process.id() as i32), libc::SIGKILL);
            }
            let _ = process.kill();
            let _ = process.wait();
            break Err(anyhow::anyhow!(
                "query timed out or shutting down ({}s limit)",
                timeout.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(20));
    };
    // Close inherited pipes held by descendants after a query exits.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(process.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    drop(query_job);
    let output = out
        .join()
        .map_err(|_| anyhow::anyhow!("stdout reader failed"))??;
    let error = err
        .join()
        .map_err(|_| anyhow::anyhow!("stderr reader failed"))??;
    let timed_out = result.is_err();
    let status = if allow_partial {
        result.ok()
    } else {
        Some(result?)
    };
    anyhow::ensure!(
        allow_partial || status.is_some_and(|s| s.success()),
        "{}",
        String::from_utf8_lossy(&error)
            .lines()
            .next()
            .unwrap_or("query failed")
    );
    let mut output = String::from_utf8_lossy(&output).into_owned();
    if include_stderr && !error.is_empty() {
        output.push_str("\n[stderr]\n");
        output.push_str(&String::from_utf8_lossy(&error));
    }
    if timed_out {
        output.push_str("\n[query ended at timeout; partial capture]\n");
    }
    Ok(output)
}

fn field(value: &Value, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|key| value.get(*key))
        .map(|v| {
            v.as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| v.to_string())
        })
        .unwrap_or_default()
}

pub fn parse_containers(runtime: &str, output: &str) -> Vec<Container> {
    output
        .lines()
        .filter_map(|line| {
            let value: Value = serde_json::from_str(line).ok()?;
            let id = field(&value, &["ID", "Id", "id"]);
            if id.is_empty() {
                return None;
            }
            Some(Container {
                id,
                runtime: runtime.into(),
                name: field(&value, &["Names", "Name"]),
                image: field(&value, &["Image"]),
                state: field(&value, &["State"]),
                status: field(&value, &["Status"]),
                ports: field(&value, &["Ports"]),
                command: field(&value, &["Command"]),
                created: field(&value, &["CreatedAt", "Created"]),
                running_for: field(&value, &["RunningFor"]),
                ..Container::default()
            })
        })
        .collect()
}

pub fn apply_container_stats(containers: &mut [Container], output: &str) {
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let id = field(&value, &["ID", "Id", "ContainerID", "Container"]);
        let name = field(&value, &["Name"]);
        if let Some(container) = containers.iter_mut().find(|c| {
            (!id.is_empty() && (c.id.starts_with(&id) || id.starts_with(&c.id)))
                || (!name.is_empty() && c.name == name)
        }) {
            container.cpu = field(&value, &["CPUPerc", "CPU", "CPUPercent"])
                .trim_end_matches('%')
                .trim()
                .parse()
                .ok();
            let memory = field(&value, &["MemUsage", "MemUsageBytes"]);
            if let Some((used, total)) = memory.split_once('/') {
                container.memory = parse_size(used);
                container.memory_limit = parse_size(total);
            }
            let network = field(&value, &["NetIO"]);
            if let Some((rx, tx)) = network.split_once('/') {
                container.network_rx = parse_size(rx);
                container.network_tx = parse_size(tx);
            }
            let block = field(&value, &["BlockIO"]);
            if let Some((read, write)) = block.split_once('/') {
                container.block_read = parse_size(read);
                container.block_write = parse_size(write);
            }
            container.memory_percent = field(&value, &["MemPerc", "MemPercent"])
                .trim_end_matches('%')
                .trim()
                .parse()
                .ok();
            container.pids = field(&value, &["PIDs", "PIDS"]).parse().ok();
        }
    }
}

pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let end = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(text.len());
    let value: f64 = text[..end].parse().ok()?;
    let multiplier = match text[end..].trim().to_ascii_lowercase().as_str() {
        "b" | "" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        "kib" | "ki" => 1024.0,
        "mib" | "mi" => 1024f64.powi(2),
        "gib" | "gi" => 1024f64.powi(3),
        "tib" | "ti" => 1024f64.powi(4),
        _ => return None,
    };
    if value.is_finite() && value >= 0.0 {
        Some((value * multiplier).round() as u64)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn discover_services(inventory: &mut Inventory) {
    if !std::path::Path::new("/run/systemd/system").exists() {
        return;
    }
    for scope in ["system", "user"] {
        if let Ok(services) = crate::systemd::services(scope) {
            inventory.service_managers.push(format!("systemd/{scope}"));
            inventory.services.extend(services);
            continue;
        }
        let mode = format!("--{scope}");
        match command(
            "systemctl",
            &[
                &mode,
                "list-units",
                "--type=service",
                "--all",
                "--output=json",
                "--no-pager",
                "--plain",
            ],
            Duration::from_secs(3),
        ) {
            Ok(output) => match serde_json::from_str::<Vec<Value>>(&output) {
                Ok(values) => {
                    inventory.service_managers.push(format!("systemd/{scope}"));
                    inventory.services.extend(values.iter().map(|v| Service {
                        name: field(v, &["unit"]),
                        scope: scope.into(),
                        load: field(v, &["load"]),
                        active: field(v, &["active"]),
                        sub: field(v, &["sub"]),
                        description: field(v, &["description"]),
                        ..Service::default()
                    }));
                }
                Err(error) => inventory
                    .notes
                    .push(format!("systemd/{scope}: invalid JSON: {error}")),
            },
            Err(error) => inventory.notes.push(format!("systemd/{scope}: {error}")),
        }
        if let Ok(output) = command(
            "systemctl",
            &[
                &mode,
                "show",
                "--all",
                "--property=Id,MainPID,MemoryCurrent,CPUUsageNSec,TasksCurrent,NRestarts,ControlGroup,FragmentPath",
                "--",
                "*.service",
            ],
            Duration::from_secs(3),
        ) {
            for unit in output.split("\n\n") {
                let fields: std::collections::BTreeMap<_, _> =
                    unit.lines().filter_map(|l| l.split_once('=')).collect();
                let Some(name) = fields.get("Id") else {
                    continue;
                };
                if let Some(service) = inventory
                    .services
                    .iter_mut()
                    .find(|s| s.scope == scope && s.name == *name)
                {
                    service.pid = fields
                        .get("MainPID")
                        .and_then(|s| s.parse().ok())
                        .filter(|n| *n > 0);
                    service.memory = fields
                        .get("MemoryCurrent")
                        .and_then(|s| s.parse().ok())
                        .filter(|n| *n < u64::MAX);
                    service.cpu_time_ns = fields
                        .get("CPUUsageNSec")
                        .and_then(|s| s.parse().ok())
                        .filter(|n| *n < u64::MAX);
                    service.tasks = fields
                        .get("TasksCurrent")
                        .and_then(|s| s.parse().ok())
                        .filter(|n| *n < u64::MAX);
                    for (key, value) in fields {
                        service.extra.insert(key.into(), serde_json::json!(value));
                    }
                }
            }
        }
        if inventory
            .service_managers
            .contains(&format!("systemd/{scope}"))
        {
            match command(
                "systemctl",
                &[
                    &mode,
                    "list-unit-files",
                    "--type=service",
                    "--output=json",
                    "--no-pager",
                ],
                Duration::from_secs(3),
            ) {
                Ok(output) => match serde_json::from_str::<Vec<Value>>(&output) {
                    Ok(values) => {
                        for value in values {
                            let name = field(&value, &["unit_file"]);
                            if !inventory
                                .services
                                .iter()
                                .any(|service| service.scope == scope && service.name == name)
                            {
                                inventory.services.push(Service {
                                    name,
                                    scope: scope.into(),
                                    load: "installed".into(),
                                    active: "not-loaded".into(),
                                    sub: field(&value, &["state"]),
                                    description: "Unit file; not currently loaded".into(),
                                    ..Service::default()
                                });
                            }
                        }
                    }
                    Err(error) => inventory
                        .notes
                        .push(format!("systemd/{scope} unit files: {error}")),
                },
                Err(error) => inventory
                    .notes
                    .push(format!("systemd/{scope} unit files: {error}")),
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn discover_services(inventory: &mut Inventory) {
    match command("launchctl", &["list"], Duration::from_secs(3)) {
        Ok(output) => {
            inventory.service_managers.push("launchd/user".into());
            inventory.services = output
                .lines()
                .skip(1)
                .filter_map(|line| {
                    let parts: Vec<_> = line.split_whitespace().collect();
                    if parts.len() < 3 {
                        return None;
                    }
                    Some(Service {
                        name: parts[2].into(),
                        scope: "user".into(),
                        load: "loaded".into(),
                        active: if parts[0] != "-" {
                            "active"
                        } else if parts[1] != "0" {
                            "failed"
                        } else {
                            "inactive"
                        }
                        .into(),
                        sub: format!("PID {} / exit {}", parts[0], parts[1]),
                        description: String::new(),
                        pid: parts[0].parse().ok(),
                        ..Service::default()
                    })
                })
                .collect();
        }
        Err(error) => inventory.notes.push(format!("launchd: {error}")),
    }
}

#[cfg(target_os = "windows")]
fn discover_services(inventory: &mut Inventory) {
    match command(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; @(Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,ProcessId,PathName,StartMode) | ConvertTo-Json -Compress",
        ],
        Duration::from_secs(5),
    ) {
        Ok(output) => {
            if let Ok(values) =
                serde_json::from_str::<Vec<Value>>(output.trim_start_matches('\u{feff}'))
            {
                inventory.service_managers.push("Windows SCM".into());
                inventory.services = values
                    .iter()
                    .map(|v| Service {
                        name: field(v, &["Name"]),
                        scope: "system".into(),
                        load: "installed".into(),
                        active: if field(v, &["State"]) == "Running" {
                            "active"
                        } else {
                            "inactive"
                        }
                        .into(),
                        sub: field(v, &["State"]),
                        extra: std::collections::BTreeMap::from([("SCM".into(), v.clone())]),
                        description: field(v, &["DisplayName"]),
                        pid: v["ProcessId"]
                            .as_u64()
                            .filter(|pid| *pid > 0)
                            .map(|pid| pid as u32),
                        ..Service::default()
                    })
                    .collect();
            } else {
                inventory
                    .notes
                    .push("Windows services: invalid JSON".into());
            }
        }
        Err(error) => inventory.notes.push(format!("Windows services: {error}")),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn discover_services(_: &mut Inventory) {}

fn listeners() -> Vec<Listener> {
    #[cfg(target_os = "linux")]
    {
        let mut result = vec![];
        for (path, protocol) in [
            ("tcp", "TCP"),
            ("tcp6", "TCP6"),
            ("udp", "UDP"),
            ("udp6", "UDP6"),
        ] {
            if let Ok(text) = std::fs::read_to_string(format!("/proc/net/{path}")) {
                result.extend(parse_listeners(&text, protocol));
            }
        }
        result.sort_by(|a, b| a.port.cmp(&b.port).then(a.protocol.cmp(&b.protocol)));
        result
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec![]
    }
}

pub fn parse_listeners(text: &str, protocol: &str) -> Vec<Listener> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() < 8
                || (protocol.starts_with("TCP") && f[3] != "0A")
                || (protocol.starts_with("UDP") && f[3] != "07")
            {
                return None;
            }
            let (ip, port) = f[1].split_once(':')?;
            let address = if ip.len() == 8 {
                let value = u32::from_str_radix(ip, 16).ok()?;
                std::net::Ipv4Addr::from(value.to_le_bytes()).to_string()
            } else if ip.len() == 32 {
                let mut bytes = [0u8; 16];
                for (i, chunk) in ip.as_bytes().as_chunks::<8>().0.iter().enumerate() {
                    bytes[i * 4..i * 4 + 4].copy_from_slice(
                        &u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16)
                            .ok()?
                            .to_le_bytes(),
                    );
                }
                std::net::Ipv6Addr::from(bytes).to_string()
            } else {
                return None;
            };
            Some(Listener {
                protocol: protocol.into(),
                address,
                port: u16::from_str_radix(port, 16).ok()?,
                uid: f[7].parse().ok()?,
            })
        })
        .collect()
}
