//! Evidence-based explanations, kept with the resource being investigated.
use crate::{
    app::{App, View},
    model::{bytes, percent},
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub domain: &'static str,
    pub severity: &'static str,
    pub title: String,
    pub evidence: String,
    pub meaning: &'static str,
    pub next: &'static str,
}
fn finding(
    domain: &'static str,
    severity: &'static str,
    title: impl Into<String>,
    evidence: String,
    meaning: &'static str,
    next: &'static str,
) -> Finding {
    Finding {
        domain,
        severity,
        title: title.into(),
        evidence,
        meaning,
        next,
    }
}
pub fn findings(app: &App) -> Vec<Finding> {
    let s = &app.snapshot;
    let mut out = vec![];
    if s.timestamp == 0 {
        return vec![finding(
            "system",
            "UNKNOWN",
            "Waiting for a sample",
            "No host sample has arrived".into(),
            "There is no evidence to diagnose yet.",
            "Wait for collection to finish.",
        )];
    }
    if s.cpu_percent >= app.config.cpu_warning {
        let top = s
            .processes
            .iter()
            .filter(|p| !p.thread)
            .max_by(|a, b| a.cpu.total_cmp(&b.cpu));
        out.push(finding("cpu", "CHECK", "CPU is busy", format!("Host {:.1}% across {} logical cores; largest process: {}", s.cpu_percent, s.cores.len(), top.map(|p| format!("{} (PID {}) {:.1}% of one core", p.name,p.pid,p.cpu)).unwrap_or_else(|| "unavailable".into())), "High CPU can be useful work. A single sample does not prove a runaway process.", "Processes: P sorts CPU; Enter shows the owner and command; compare its trend before adjusting priority."));
    }
    let hot_cores = s.cores.iter().filter(|v| **v >= 95.).count();
    if hot_cores > 0 && s.cpu_percent < app.config.cpu_warning {
        out.push(finding(
            "cpu",
            "CHECK",
            "Some cores are saturated",
            format!(
                "{hot_cores}/{} cores >=95%; host {:.1}%",
                s.cores.len(),
                s.cpu_percent
            ),
            "Single-threaded work can be CPU-limited even when total CPU looks low.",
            "Processes: P, then Enter; inspect the busy process and its thread view (H).",
        ));
    }
    if let Some(wait) = s.cpu_breakdown.get("iowait").filter(|v| **v >= 10.) {
        out.push(finding("storage", "CHECK", "Tasks are waiting for I/O", format!("CPU I/O wait {wait:.1}%"), "The CPU is waiting for data. Device latency, a remote filesystem or memory pressure may be involved.", "Storage: compare AWAIT and QUEUE; Processes: inspect disk read/write rates and blocked processes."));
    }
    if let Some(steal) = s.cpu_breakdown.get("steal").filter(|v| **v >= 5.) {
        out.push(finding(
            "cpu",
            "CHECK",
            "Virtual machine CPU contention",
            format!("CPU steal {steal:.1}%"),
            "The hypervisor withheld CPU time from this guest.",
            "Check host/VM CPU allocation with the machine operator before tuning the application.",
        ));
    }
    if s.memory.total > 0 && percent(s.memory.available, s.memory.total) < 10. {
        let top = s
            .processes
            .iter()
            .filter(|p| !p.thread)
            .max_by_key(|p| p.memory);
        out.push(finding("memory", "CHECK", "Little available memory", format!("{} available of {}; swap {} / {}; largest RSS: {}",bytes(s.memory.available as f64),bytes(s.memory.total as f64),bytes(s.memory.swap_used as f64),bytes(s.memory.swap_total as f64), top.map(|p| format!("{} PID {} {}",p.name,p.pid,bytes(p.memory as f64))).unwrap_or_else(|| "unknown".into())), "Low available memory can trigger reclaim, swapping or OOM. Used/cache memory alone is not proof of a leak.", "Processes: M sorts memory; compare the selected process trend. Use m > Kernel/OOM messages to check actual kills."));
    }
    for psi in &s.pressure {
        if psi.some_10.is_some_and(|v| v >= 10.) || psi.full_10.is_some_and(|v| v >= 1.) {
            out.push(finding(match psi.resource.as_str() { "cpu" => "cpu", "memory" => "memory", _ => "storage" }, "CHECK", format!("{} resource stalls",psi.resource), format!("PSI some/10s {:?}%, full/10s {:?}%",psi.some_10,psi.full_10), "PSI measures time when tasks could not make progress. It is different from resource utilization.", "Compare this resource's trend with its busiest processes and the incident time."));
        }
    }
    for disk in s
        .disks
        .iter()
        .filter(|d| d.total > 0 && percent(d.used, d.total) >= 90.)
    {
        out.push(finding("storage", if percent(disk.used,disk.total) >= 95. { "URGENT" } else { "CHECK" }, format!("{} is nearly full",disk.mount), format!("{:.1}% used; {} available, {} total",percent(disk.used,disk.total),bytes(disk.available as f64),bytes(disk.total as f64)), "Writes, logs, builds and databases can fail when this filesystem runs out of space.", "Storage: U finds large files/folders without deleting anything. Check inodes and deleted-open files with m > Storage checks."));
    }
    for disk in &s.disks {
        if let Some((total, free)) = disk
            .extra
            .get("inodes_total")
            .and_then(|v| v.as_u64())
            .zip(disk.extra.get("inodes_free").and_then(|v| v.as_u64()))
            && total > 0
            && percent(free, total) < 10.
        {
            out.push(finding("storage","CHECK",format!("{} has few free inodes",disk.mount),format!("{free} free of {total} inodes"),"New files need inodes even if free bytes remain.","Storage: U locates large folder trees. Check file counts and log/build retention before removing anything."));
        }
        if disk
            .extra
            .get("readonly")
            .or_else(|| disk.extra.get("read_only"))
            .and_then(|v| v.as_bool())
            == Some(true)
        {
            out.push(finding("storage","INFO",format!("{} is read-only",disk.mount),"Filesystem reports read_only=true".into(),"Writes here will fail. Some system/image mounts are intentionally read-only.","Check the intended mount configuration and kernel/storage errors before attempting a remount."));
        }
    }
    for disk in s
        .disk_io
        .iter()
        .filter(|d| d.await_ms.is_some_and(|v| v >= 50.))
    {
        out.push(finding("storage", "CHECK",format!("{} has slow I/O",disk.name),format!("AWAIT {:.1} ms; queue {:?}; busy {:?}%",disk.await_ms.unwrap_or_default(),disk.queue_depth,disk.busy_percent), "I/O requests are taking time to finish; acceptable latency depends on the device and workload.", "Compare QUEUE and throughput over time. Processes: look for heavy readers/writers and blocked tasks; check device health before deleting files."));
    }
    for gpu in &s.gpus {
        if gpu.memory_total.is_some_and(|v| v > 0)
            && gpu
                .memory_used
                .zip(gpu.memory_total)
                .is_some_and(|(u, t)| percent(u, t) >= 90.)
        {
            out.push(finding("gpu","CHECK",format!("GPU {} memory is nearly full",gpu.id),format!("{}: {} / {}",gpu.name,bytes(gpu.memory_used.unwrap_or_default() as f64),bytes(gpu.memory_total.unwrap_or_default() as f64)),"New GPU allocations may fail. High VRAM use alone does not show a memory leak.","GPU: inspect VRAM owners. Lower your job's batch/context size, or select a device with enough free memory; don't terminate unrelated jobs."));
        }
        if gpu
            .temperature
            .is_some_and(|t| t >= app.config.gpu_temperature_warning)
        {
            out.push(finding("gpu","CHECK",format!("GPU {} is hot",gpu.id),format!("{}°C; warning {}°C",gpu.temperature.unwrap_or_default(),app.config.gpu_temperature_warning),"High temperature can reduce clock speeds. The temperature threshold is a configured warning, not proof of hardware throttling.","GPU Device info: inspect throttle reasons, clocks and power. Check cooling and workload before changing device settings."));
        }
    }
    for n in &s.networks {
        if n.errors > 0 || n.drops.is_some_and(|d| d > 0) {
            out.push(finding("network","INFO",format!("{} has recorded errors/drops",n.name),format!("Lifetime errors {}; drops {:?}",n.errors,n.drops),"These are cumulative counters; they may predate the current incident.","Compare successive samples. Check link/cable/Wi-Fi quality only if counters keep increasing; N checks a website or service."));
        }
    }
    let pending = s
        .connections
        .connections
        .iter()
        .filter(|c| c.state == "SYN_SENT")
        .count();
    let closing = s
        .connections
        .connections
        .iter()
        .filter(|c| c.state == "CLOSE_WAIT")
        .count();
    if pending > 0 {
        out.push(finding("network","CHECK","Connections waiting for a reply",format!("{pending} sockets in SYN_SENT"),"A destination has not yet accepted the TCP connection. A short transition can be normal.","Network: filter state:SYN_SENT, Enter for the destination/owner, then N to test it. Check route, firewall and server listener if this persists."));
    }
    if closing >= 10 {
        out.push(finding("network","CHECK","Many peer-closed connections",format!("{closing} sockets in CLOSE_WAIT"),"Peers closed these sockets, while local applications still hold them. Accumulation may indicate missing socket cleanup.","Network: filter state:CLOSE_WAIT and sort by process. Inspect that application's logs and socket lifecycle."));
    }
    for p in s
        .processes
        .iter()
        .filter(|p| !p.thread && (p.state == "Zombie" || p.state == "Z"))
    {
        out.push(finding("process","CHECK",format!("{} is a zombie",p.name),format!("PID {}, parent {:?}",p.pid,p.parent),"The process exited; its parent has not collected its exit status. Killing the zombie will not reclaim it.","Processes: enable tree (t), inspect the parent and its logs. Fix child reaping in the parent application."));
        if out.len() >= 100 {
            break;
        }
    }
    for c in &s.inventory.containers {
        if c.state == "restarting"
            || c.status.to_lowercase().contains("unhealthy")
            || c.state == "dead"
        {
            out.push(finding("container","CHECK",format!("Container {} needs attention",c.name),format!("{} · {} · {}",c.runtime,c.state,c.status),"A restart loop or failing health check may make the application unavailable.","Containers: Enter shows exit code, OOM flag, restart count, health output, limits and mounts; l shows logs. Inspect the cause before restarting."));
        }
        if c.memory
            .zip(c.memory_limit)
            .is_some_and(|(u, t)| t > 0 && percent(u, t) >= 90.)
        {
            out.push(finding("container","CHECK",format!("Container {} near memory limit",c.name),format!("{} / {}",bytes(c.memory.unwrap_or_default() as f64),bytes(c.memory_limit.unwrap_or_default() as f64)),"The container can be OOM-killed even if the host has free RAM.","Inspect its actual OOM/exit status with Enter, check logs, and compare the workload size with its configured limit."));
        }
    }
    for service in &s.inventory.services {
        if service.active == "failed" {
            out.push(finding("service","CHECK",format!("Service {} failed",service.name),format!("scope {} · {} / {}",service.scope,service.active,service.sub),"The service manager reports a failed unit; this does not identify the underlying error.","Services: l opens logs; Enter shows exit status, restart count, dependencies and config path. Inspect these before choosing restart in m."));
        }
    }
    out.sort_by_key(|f| match f.severity {
        "URGENT" => 0,
        "CHECK" => 1,
        "UNKNOWN" => 2,
        _ => 3,
    });
    out.truncate(100);
    out
}
pub fn relevant(view: View, domain: &str) -> bool {
    match view {
        View::Overview | View::System | View::Explorer | View::History | View::Events => true,
        View::Processes => ["cpu", "memory", "process", "storage"].contains(&domain),
        View::Gpu => domain == "gpu",
        View::Storage => domain == "storage",
        View::Network => domain == "network",
        View::Containers => domain == "container",
        View::Services => domain == "service",
    }
}
fn text_domain(text: &str) -> &'static str {
    let text = text.to_lowercase();
    if text.contains("container ") {
        "container"
    } else if text.contains("service ") {
        "service"
    } else if text.contains("gpu") {
        "gpu"
    } else if text.contains("network") || text.contains("interface") {
        "network"
    } else if text.contains("disk") || text.contains("storage") || text.contains("io pressure") {
        "storage"
    } else if text.contains("memory") || text.contains("ram") || text.contains("swap") {
        "memory"
    } else if text.contains("cpu") {
        "cpu"
    } else {
        "system"
    }
}
pub fn related_alert(app: &App) -> Option<&str> {
    app.snapshot
        .alerts
        .iter()
        .find(|text| relevant(app.view, text_domain(text)))
        .map(String::as_str)
}
pub fn summary(app: &App) -> String {
    let findings = findings(app);
    let related: Vec<_> = findings
        .iter()
        .filter(|f| relevant(app.view, f.domain))
        .collect();
    let checks = related
        .iter()
        .filter(|f| f.severity == "CHECK" || f.severity == "URGENT")
        .count();
    let description = if let Some(first) = related.first() {
        first.title.clone()
    } else {
        "No threshold finding in this sample".into()
    };
    let last = app
        .events
        .history
        .iter()
        .rev()
        .find(|e| relevant(app.view, text_domain(&e.condition)));
    format!(
        " {checks} checks · {description}{}",
        last.map(|e| format!(
            " · {}: {}",
            if e.active { "Alert" } else { "Recovered" },
            e.condition
        ))
        .unwrap_or_default()
    )
}
fn trends(app: &App) -> String {
    let mut body = String::from("RECENT TRENDS · current / minimum / peak / average\n");
    let end = app.history.timestamps.back().copied().unwrap_or(0.);
    let start = if app.config.time_window_seconds == 0 {
        f64::NEG_INFINITY
    } else {
        end - app.config.time_window_seconds as f64
    };
    let skip = app
        .history
        .timestamps
        .iter()
        .take_while(|t| **t < start)
        .count();
    let mut count = 0;
    for (key, values) in &app.history.series {
        if !(relevant(app.view, text_domain(key))
            || (app.view == View::Gpu
                && app.snapshot.gpus.iter().any(|g| key.starts_with(&g.name))))
        {
            continue;
        }
        let offset = app.history.timestamps.len().saturating_sub(values.len());
        let values: Vec<_> = values
            .iter()
            .skip(skip.saturating_sub(offset))
            .filter(|v| v.is_finite())
            .copied()
            .collect();
        if values.is_empty() {
            continue;
        }
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let peak = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let average = values.iter().sum::<f64>() / values.len() as f64;
        body.push_str(&format!(
            "{key}: {:.1} / {min:.1} / {peak:.1} / {average:.1} ({} samples)\n",
            values.last().unwrap_or(&0.),
            values.len()
        ));
        count += 1;
        if count >= 30 {
            break;
        }
    }
    if count == 0 {
        body.push_str("No retained samples for this resource yet.\n");
    }
    let span = app
        .history
        .timestamps
        .back()
        .zip(app.history.timestamps.front())
        .map_or(0., |(b, a)| b - a);
    body.push_str(&format!("Retained duration: {span:.1}s. Sampled peaks can miss activity between samples. [F5] View changes the displayed window.\n"));
    body
}
pub fn report(app: &App) -> String {
    let mut body = format!(
        "{} · DIAGNOSIS\nHost {} · sample timestamp {} · {}\nThese are evidence-based checks, not confirmed root causes.\n\n",
        app.view.name(),
        app.snapshot.hostname,
        app.snapshot.timestamp,
        if app.replay {
            "recorded data only"
        } else {
            "current sampled data"
        }
    );
    let related: Vec<_> = findings(app)
        .into_iter()
        .filter(|f| relevant(app.view, f.domain))
        .collect();
    if related.is_empty() {
        body.push_str("No configured threshold finding in this sample. This does not prove that the resource or application is healthy.\n\n");
    }
    for f in related {
        body.push_str(&format!(
            "[{}] {}\nEvidence: {}\nWhat it means: {}\nNext: {}\n\n",
            f.severity, f.title, f.evidence, f.meaning, f.next
        ));
    }
    body.push_str(match app.view {
        View::Network => "HOW TO INVESTIGATE\nCan't open a website? [F6] Tools > Test website or service checks DNS/TCP/TLS/HTTP and compares direct access with environment proxy.\nPort already in use? [F3] Filter port:3000, then [Enter] to see the owner.\nSlow connection? [F4] Sort or click a column header; compare RTT/queues and inspect errors.\nNeed packet evidence? [F6] Tools > Capture packets creates a short PCAP; Open a packet capture previews headers.\nConnections and interfaces share one page; click a table or [F5] View to focus it. UDP byte rates, encrypted hostnames and proxy rules are not inferred.\n\n",
        View::Storage => "HOW TO INVESTIGATE\nDisk full? [F6] Tools > Find large files and folders scans without deleting.\nFree space but writes fail? Tools > Inodes and deleted-open files checks inodes, read-only mounts and deleted-open files.\nSlow builds/database? Compare throughput, AWAIT (request latency) and QUEUE (pending work), then inspect process I/O.\n\n",
        View::Processes => "HOW TO INVESTIGATE\n[F4] Sort by CPU or RAM; select a process to see its own trend.\n[Enter] shows command, working directory, cgroup and limits.\n[F6] Tools finds output/logs, environment and open files. [F5] View selects the process tree.\nAdvanced process tools includes locks and trace/backtrace. Attaching may briefly pause a process and requires confirmation.\n\n",
        View::Gpu => "HOW TO INVESTIGATE\nGPU processes, utilization/power/temperature trends and device information share one page. Click a table or [F5] View to focus it.\nDevice information shows driver/CUDA support, clocks, ECC and supported throttle reasons.\nHigh VRAM with low compute can be model residency, data loading or a stalled job; check process logs and CPU/storage too.\nMissing driver counters remain unknown.\n\n",
        View::Containers => "HOW TO INVESTIGATE\n[Enter]: exit/OOM/health/restarts, resource limits, mounts and published ports.\n[F6] Tools: recent logs and management, including processes, filesystem changes, inspect and confirmed lifecycle actions.\nAn exposed image port alone does not mean a host port is published.\n\n",
        View::Services => "HOW TO INVESTIGATE\n[Enter]: exit result, restart count, dependencies, config path and resource limits.\n[F6] Tools: recent logs and management, including status/config/dependencies and confirmed lifecycle actions.\nThe unit being active does not prove that the application endpoint is healthy; Network > Tools > Test website or service tests it.\n\n",
        _ => "HOW TO INVESTIGATE\nCPU or RAM pressure: Processes identifies owners and logs.\nGPU failures: GPU identifies memory owners, driver status and hardware signals.\nDisk full/slow: Storage checks capacity, latency, inodes and large files.\nCan't connect/port conflict: Network identifies connections and tests a target.\nContainer/service failures: open that resource's logs and exit/health details before restarting.\n[F6] Tools lists available actions and additional details for this resource.\n\n",
    });
    body.push_str(&trends(app));
    body.push_str("\nRELATED ALERTS & RECOVERIES · since this monitoring session started\n");
    let mut count = 0;
    for event in app
        .events
        .history
        .iter()
        .rev()
        .filter(|e| relevant(app.view, text_domain(&e.condition)))
        .take(20)
    {
        body.push_str(&format!(
            "{} · {}s ago · {}\n",
            if event.active { "ALERT" } else { "RECOVERED" },
            app.snapshot.timestamp.saturating_sub(event.timestamp),
            event.condition
        ));
        count += 1;
    }
    if count == 0 {
        body.push_str("No recorded alert transitions for this resource. Short spikes may not satisfy the alert hold time.\n");
    }
    body
}
pub fn sampled_details(app: &App) -> String {
    let mut body = format!(
        "{} · ADDITIONAL SAMPLED DETAILS\nRead-only observations; unavailable fields are not zero.\n\n",
        app.view.name()
    );
    for (key, value) in app
        .snapshot
        .extra
        .iter()
        .chain(app.snapshot.inventory.extra.iter())
    {
        let domain = text_domain(key);
        if relevant(app.view, domain) || (matches!(app.view, View::System | View::Overview)) {
            body.push_str(&format!("{key}\n{}\n\n", crate::fields::pretty(value)));
        }
    }
    body
}

/// Bounded single traversal; report partial results instead of treating unreadable paths as empty.
pub fn folder_report(path: &std::path::Path) -> anyhow::Result<String> {
    use std::{
        collections::HashMap,
        fs,
        time::{Duration, Instant},
    };
    let root = path.canonicalize()?;
    anyhow::ensure!(root.is_dir(), "Choose a folder");
    let began = Instant::now();
    let root_meta = fs::metadata(&root)?;
    let mut todo = vec![(root.clone(), String::new())];
    let (mut count, mut errors, mut skipped, mut complete) = (0usize, 0usize, 0usize, true);
    let mut sizes = HashMap::<String, u64>::new();
    let mut largest = vec![];
    while let Some((path, mut group)) = todo.pop() {
        if count >= 100_000 || began.elapsed() > Duration::from_secs(2) {
            complete = false;
            break;
        }
        count += 1;
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        if meta.is_symlink() {
            skipped += 1;
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.dev() != root_meta.dev() {
                skipped += 1;
                continue;
            }
        }
        #[cfg(not(unix))]
        let _ = &root_meta;
        if meta.is_dir() {
            match fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        if count + todo.len() >= 100_000 || began.elapsed() > Duration::from_secs(2)
                        {
                            complete = false;
                            break;
                        }
                        match entry {
                            Ok(entry) => {
                                if path == root {
                                    group = entry.file_name().to_string_lossy().into_owned();
                                }
                                todo.push((entry.path(), group.clone()));
                            }
                            Err(_) => errors += 1,
                        }
                    }
                }
                Err(_) => errors += 1,
            }
        } else if meta.is_file() {
            *sizes.entry(group).or_default() += meta.len();
            largest.push((meta.len(), path));
            if largest.len() > 200 {
                largest.sort_by_key(|(size, _)| std::cmp::Reverse(*size));
                largest.truncate(30);
            }
        }
    }
    let mut sizes: Vec<_> = sizes.into_iter().collect();
    sizes.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    largest.sort_by_key(|(size, _)| std::cmp::Reverse(*size));
    largest.truncate(30);
    let mut body = format!(
        "FOLDER SPACE · {}\n{} · {count} entries examined · {errors} unreadable · {skipped} symlinks/other filesystems skipped\nLogical file lengths, not allocated disk blocks; hard links may count more than once. No files deleted.\n\nLARGEST TOP-LEVEL ENTRIES\n",
        root.display(),
        if complete && errors == 0 {
            "Complete"
        } else {
            "PARTIAL RESULTS (2s / 100,000 entries limit)"
        }
    );
    for (name, size) in sizes.iter().take(30) {
        body.push_str(&format!("{:>14}  {name}\n", bytes(*size as f64)));
    }
    body.push_str("\nLARGEST FILES FOUND\n");
    for (size, path) in largest {
        body.push_str(&format!("{:>14}  {}\n", bytes(size as f64), path.display()));
    }
    Ok(body)
}
pub fn kernel_messages() -> String {
    #[cfg(target_os = "linux")]
    {
        match crate::discovery::command(
            "journalctl",
            &[
                "-k",
                "--since",
                "-2h",
                "-n",
                "1000",
                "--no-pager",
                "-o",
                "short-iso",
            ],
            std::time::Duration::from_secs(4),
        ) {
            Ok(output) => {
                let lines: Vec<_> = output
                    .lines()
                    .filter(|line| {
                        let l = line.to_lowercase();
                        [
                            "oom",
                            "out of memory",
                            "killed process",
                            "error",
                            "fail",
                            "reset",
                            "xid",
                            "thermal",
                            "nvme",
                            "i/o",
                        ]
                        .iter()
                        .any(|term| l.contains(term))
                    })
                    .collect();
                format!(
                    "KERNEL / OOM · matching lines from last 2h (last 1000 kernel entries)\nActual kernel messages, not inferred causes. Access depends on journal permissions.\n\n{}",
                    if lines.is_empty() {
                        "No matching messages in the accessible sample. This does not rule out an earlier or inaccessible fault.".into()
                    } else {
                        lines.join("\n")
                    }
                )
            }
            Err(error) => format!(
                "Cannot read kernel journal: {error}\nAsk the operator for access to kernel/OOM logs; no permission changes were made."
            ),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        "Kernel/OOM message lookup currently uses the Linux journal. Use the platform system log viewer; resource diagnosis and sampled trends remain available.".into()
    }
}
pub fn storage_checks(app: &App) -> String {
    let mut body = String::from(
        "STORAGE WRITE FAILURES\nFree bytes and free inodes are separate: every new file needs an inode. A deleted file still held open can keep space allocated.\n\n",
    );
    for d in &app.snapshot.disks {
        body.push_str(&format!(
            "{} · {} available · {}\n{}\n\n",
            d.mount,
            bytes(d.available as f64),
            d.filesystem,
            crate::fields::pretty(&serde_json::json!(d.extra))
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let began = std::time::Instant::now();
        let (mut count, mut denied, mut partial) = (0usize, 0usize, false);
        body.push_str("DELETED-OPEN REGULAR FILES · readable processes only\n");
        'scan: for p in &app.snapshot.processes {
            if p.thread {
                continue;
            }
            if began.elapsed() > std::time::Duration::from_secs(1) {
                partial = true;
                break;
            }
            let Ok(entries) = std::fs::read_dir(format!("/proc/{}/fd", p.pid)) else {
                denied += 1;
                continue;
            };
            for entry in entries.flatten() {
                if began.elapsed() > std::time::Duration::from_secs(1) || count >= 100 {
                    partial = true;
                    break 'scan;
                }
                if let Ok(link) = std::fs::read_link(entry.path())
                    && link.to_string_lossy().ends_with(" (deleted)")
                    && let Ok(meta) = std::fs::metadata(entry.path())
                    && meta.is_file()
                {
                    body.push_str(&format!(
                        "PID {} {} FD {} · {} · {}\n",
                        p.pid,
                        p.name,
                        entry.file_name().to_string_lossy(),
                        bytes(meta.len() as f64),
                        link.display()
                    ));
                    count += 1;
                }
            }
        }
        body.push_str(&format!("{count} entries shown (same file can appear more than once); {denied} processes unreadable; scan {}.\nRelease files through the owning application's normal log rotation/reopen mechanism.\n",if partial {"PARTIAL (1s / 100 results limit)"} else {"finished"}));
    }
    body
}

pub fn resource_alerts(s: &crate::model::Snapshot, _config: &crate::config::Config) -> Vec<String> {
    let mut alerts = vec![];
    for d in &s.disks {
        if d.total > 0 && percent(d.used, d.total) >= 90. {
            alerts.push(format!("Storage {} nearly full", d.mount));
        }
        if d.extra
            .get("inodes_total")
            .and_then(|v| v.as_u64())
            .zip(d.extra.get("inodes_free").and_then(|v| v.as_u64()))
            .is_some_and(|(total, free)| total > 0 && percent(free, total) < 10.)
        {
            alerts.push(format!("Storage {} few free inodes", d.mount));
        }
    }
    for g in &s.gpus {
        if g.memory_used
            .zip(g.memory_total)
            .is_some_and(|(used, total)| total > 0 && percent(used, total) >= 90.)
        {
            alerts.push(format!("GPU {} memory nearly full", g.id));
        }
    }
    if s.connections.sampled_at > 0 && s.timestamp.saturating_sub(s.connections.sampled_at) <= 3 {
        if s.connections
            .connections
            .iter()
            .any(|c| c.state == "SYN_SENT")
        {
            alerts.push("Network TCP connections waiting for reply".into());
        }
        if s.connections
            .connections
            .iter()
            .filter(|c| c.state == "CLOSE_WAIT")
            .count()
            >= 10
        {
            alerts.push("Network many peer-closed connections".into());
        }
    }
    alerts.truncate(100);
    alerts
}
