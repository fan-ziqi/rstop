mod probes;
#[cfg(target_os = "linux")]
use crate::collector::rate;
pub use probes::discover_without_platform_gpu;
pub use probes::{discover, folder_size, local_http};
#[cfg(target_os = "linux")]
use std::collections::HashSet;
// Platform telemetry beyond sysinfo. Unsupported/denied probes remain explicit.
#[cfg(target_os = "linux")]
use crate::collector::unix_time;
use crate::{config::Config, model::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
#[cfg(target_os = "linux")]
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Advanced {
    #[cfg(target_os = "linux")]
    energy: HashMap<String, (u64, Instant)>,
    #[cfg(target_os = "linux")]
    host_counters: HashMap<String, u64>,
    #[cfg(target_os = "linux")]
    host_at: Option<Instant>,
    #[cfg(target_os = "linux")]
    disk: HashMap<String, Vec<u64>>,
    #[cfg(target_os = "linux")]
    disk_at: Option<Instant>,
    #[cfg(target_os = "linux")]
    drm: HashMap<(u32, String, String), (u64, Instant)>,
    #[cfg(target_os = "linux")]
    process_fields: HashMap<(u32, u64), (Instant, BTreeMap<String, Value>)>,
    #[cfg(target_os = "linux")]
    delays: crate::taskstats::Collector,
}
impl Advanced {
    pub fn collect(&mut self, s: &mut Snapshot, config: &Config) {
        #[cfg(target_os = "linux")]
        {
            let now = Instant::now();
            let text = fs::read_to_string("/proc/stat").unwrap_or_default();
            let mut counters = HashMap::new();
            let mut scheduler = BTreeMap::new();
            for (key, label) in [
                ("ctxt", "context_switches"),
                ("intr", "interrupts"),
                ("softirq", "soft_interrupts"),
                ("processes", "processes_created"),
                ("procs_running", "runnable_tasks"),
                ("procs_blocked", "blocked_tasks"),
            ] {
                if let Some(value) = text.lines().find_map(|line| {
                    let mut f = line.split_whitespace();
                    (f.next() == Some(key))
                        .then(|| f.next()?.parse::<u64>().ok())
                        .flatten()
                }) {
                    scheduler.insert(format!("{label}_total"), json!(value));
                    if !key.starts_with("procs_") {
                        scheduler.insert(
                            format!("{label}_per_second"),
                            json!(self.host_at.and_then(|at| {
                                self.host_counters
                                    .get(key)
                                    .and_then(|old| value.checked_sub(*old))
                                    .map(|delta| {
                                        delta as f64 / now.duration_since(at).as_secs_f64()
                                    })
                            })),
                        );
                    }
                    counters.insert(key.to_string(), value);
                }
            }
            let vm = parse_process_fields(
                &fs::read_to_string("/proc/vmstat")
                    .unwrap_or_default()
                    .lines()
                    .map(|l| l.replacen(' ', ": ", 1))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            for (key, label) in [("pswpin", "swap_in"), ("pswpout", "swap_out")] {
                if let Some(pages) = vm.get(key).and_then(Value::as_u64) {
                    let bytes = pages
                        .saturating_mul(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1) as u64);
                    scheduler.insert(format!("{label}_bytes"), json!(bytes));
                    scheduler.insert(
                        format!("{label}_bytes_per_second"),
                        json!(self.host_at.and_then(|at| {
                            self.host_counters
                                .get(key)
                                .and_then(|old| bytes.checked_sub(*old))
                                .map(|delta| delta as f64 / now.duration_since(at).as_secs_f64())
                        })),
                    );
                    counters.insert(key.into(), bytes);
                }
            }
            self.host_counters = counters;
            self.host_at = Some(now);
            s.extra
                .insert("CPU scheduler / swap IO".into(), json!(scheduler));

            if let Ok(entries) = fs::read_dir("/sys/class/powercap") {
                let mut power = vec![];
                for entry in entries.flatten() {
                    let p = entry.path();
                    let Some(energy) = number(p.join("energy_uj")) else {
                        continue;
                    };
                    let watts =
                        self.energy
                            .get(&p.to_string_lossy().into_owned())
                            .and_then(|(old, at)| {
                                let delta = if energy >= *old {
                                    energy - old
                                } else {
                                    number(p.join("max_energy_range_uj"))?
                                        .checked_sub(*old)?
                                        .checked_add(energy)?
                                };
                                Some(
                                    delta as f64
                                        / 1e6
                                        / now.duration_since(*at).as_secs_f64().max(0.001),
                                )
                            });
                    self.energy
                        .insert(p.to_string_lossy().into_owned(), (energy, now));
                    power.push(
                        json!({"name":read(p.join("name")),"watts":watts,"energy_uj":energy}),
                    );
                }
                if !power.is_empty() {
                    s.extra.insert("CPU power (RAPL)".into(), json!(power));
                }
            }
            let counters =
                parse_full_diskstats(&fs::read_to_string("/proc/diskstats").unwrap_or_default());
            if let Some(at) = self.disk_at {
                let seconds = now.duration_since(at).as_secs_f64();
                for d in &mut s.disk_io {
                    if let Some((new, old)) = counters.get(&d.name).zip(self.disk.get(&d.name)) {
                        let delta = |i: usize| new[i].checked_sub(old[i]);
                        d.read_iops = delta(0).map(|v| v as f64 / seconds);
                        d.write_iops = delta(4).map(|v| v as f64 / seconds);
                        let ops = delta(0).zip(delta(4)).map(|(a, b)| a + b);
                        d.await_ms = ops
                            .filter(|n| *n > 0)
                            .and_then(|n| Some((delta(3)? + delta(7)?) as f64 / n as f64));
                        d.queue_depth = delta(10).map(|v| v as f64 / 1000.0 / seconds);
                    }
                }
            }
            self.disk = counters;
            self.disk_at = Some(now);
            self.drm_processes(s, now);
            let extra_status = config.columns.iter().any(|c| {
                [
                    "uid",
                    "tgid",
                    "data",
                    "code",
                    "lib",
                    "swap",
                    "context_switches",
                    "uid_effective",
                    "uid_saved",
                    "gid",
                    "gid_effective",
                    "nspid",
                    "seccomp",
                    "capabilities",
                    "voluntary_context_switches",
                    "nonvoluntary_context_switches",
                ]
                .contains(&c.as_str())
            });
            let extra_io = config.columns.iter().any(|c| {
                ["rchar", "wchar", "syscr", "syscw", "cancelled_write"].contains(&c.as_str())
            });
            let extra_smaps = config
                .columns
                .iter()
                .any(|c| ["pss", "pss_swap", "epss", "uss"].contains(&c.as_str()));
            let extra_statm = config
                .columns
                .iter()
                .any(|c| ["shared", "private"].contains(&c.as_str()));
            let identities: HashSet<_> =
                s.processes.iter().map(|p| (p.pid, p.start_time)).collect();
            self.process_fields
                .retain(|key, _| identities.contains(key));
            let delay_requested = config
                .columns
                .iter()
                .any(|c| ["cpu_delay", "io_delay", "swap_delay"].contains(&c.as_str()));
            if delay_requested {
                self.delays
                    .retain(&s.processes.iter().map(|p| (p.pid, p.start_time)).collect());
            }
            let stat_requested = config.columns.iter().any(|c| {
                [
                    "priority",
                    "processor",
                    "threads",
                    "pgrp",
                    "session",
                    "tty",
                    "minor_faults",
                    "major_faults",
                    "utime",
                    "stime",
                    "cutime",
                    "cstime",
                    "tpgid",
                    "minor_faults_children",
                    "major_faults_children",
                    "rt_priority",
                    "scheduling",
                    "cpu_time_children",
                ]
                .contains(&c.as_str())
            });
            let delay_began = Instant::now();
            let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
            for p in &mut s.processes {
                p.extra
                    .insert("clock_ticks_per_second".into(), json!(ticks));
                let requested = extra_status
                    || extra_io
                    || extra_smaps
                    || extra_statm
                    || config.columns.iter().any(|c| {
                        [
                            "oom",
                            "io_priority",
                            "cgroup",
                            "autogroup",
                            "autogroup_nice",
                            "cpu_delay",
                            "io_delay",
                            "swap_delay",
                            "comm",
                            "exe",
                            "cwd",
                            "security",
                            "wchan",
                            "exe_deleted",
                            "fd_count",
                            "autogroup_id",
                        ]
                        .contains(&c.as_str())
                    });
                if requested {
                    let identity = (p.pid, p.start_time);
                    let refresh = self
                        .process_fields
                        .get(&identity)
                        .is_none_or(|(at, _)| now.duration_since(*at) > Duration::from_secs(5));
                    if refresh {
                        let mut values = BTreeMap::new();
                        for (required, file) in [
                            (extra_status, "status"),
                            (extra_io, "io"),
                            (extra_smaps, "smaps_rollup"),
                        ] {
                            if required {
                                values.insert(
                                    file.into(),
                                    read(format!("/proc/{}/{file}", p.pid))
                                        .map(|text| json!(parse_process_fields(&text)))
                                        .unwrap_or(Value::Null),
                                );
                            }
                        }
                        if extra_statm
                            && let Some(shared) =
                                read(format!("/proc/{}/statm", p.pid)).and_then(|text| {
                                    text.split_whitespace()
                                        .nth(2)
                                        .and_then(|v| v.parse::<u64>().ok())
                                })
                        {
                            values.insert(
                                "shared_bytes".into(),
                                json!(
                                    shared
                                        * unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1)
                                            as u64
                                ),
                            );
                        }
                        for (column, file) in [
                            ("oom", "oom_score"),
                            ("cgroup", "cgroup"),
                            ("autogroup", "autogroup"),
                            ("autogroup_nice", "autogroup"),
                            ("autogroup_id", "autogroup"),
                        ] {
                            if config.columns.iter().any(|c| c == column) {
                                let text = read(format!("/proc/{}/{file}", p.pid));
                                values.insert(
                                    if column == "autogroup_id" {
                                        "autogroup"
                                    } else {
                                        column
                                    }
                                    .into(),
                                    if column == "oom" {
                                        text.and_then(|s| s.parse::<u64>().ok())
                                            .map(Value::from)
                                            .unwrap_or(Value::Null)
                                    } else if column == "autogroup_nice" {
                                        text.and_then(|s| {
                                            s.split_whitespace()
                                                .last()
                                                .and_then(|v| v.parse::<i64>().ok())
                                        })
                                        .map(Value::from)
                                        .unwrap_or(Value::Null)
                                    } else {
                                        json!(text)
                                    },
                                );
                            }
                        }
                        if config.columns.iter().any(|c| c == "io_priority") {
                            let io = unsafe { libc::syscall(libc::SYS_ioprio_get, 1, p.pid) };
                            values.insert(
                                "io_priority".into(),
                                if io < 0 {
                                    Value::Null
                                } else {
                                    json!(format!(
                                        "{}:{}",
                                        match io >> 13 {
                                            1 => "Realtime",
                                            2 => "Best effort",
                                            3 => "Idle",
                                            _ => "Inherited",
                                        },
                                        io & 8191
                                    ))
                                },
                            );
                        }
                        for (column, file) in [
                            ("comm", "comm"),
                            ("security", "attr/current"),
                            ("wchan", "wchan"),
                        ] {
                            if config.columns.iter().any(|c| c == column) {
                                values.insert(
                                    column.into(),
                                    json!(read(format!("/proc/{}/{file}", p.pid))),
                                );
                            }
                        }
                        for (column, file) in [("exe", "exe"), ("cwd", "cwd")] {
                            if config
                                .columns
                                .iter()
                                .any(|c| c == column || (column == "exe" && c == "exe_deleted"))
                            {
                                let path = fs::read_link(format!("/proc/{}/{file}", p.pid))
                                    .ok()
                                    .map(|p| p.to_string_lossy().into_owned());
                                if column == "exe" {
                                    values.insert(
                                        "exe_deleted".into(),
                                        path.as_ref()
                                            .map(|p| json!(p.ends_with(" (deleted)")))
                                            .unwrap_or(Value::Null),
                                    );
                                }
                                values.insert(column.into(), json!(path));
                            }
                        }
                        if config.columns.iter().any(|c| c == "fd_count") {
                            let count = fs::read_dir(format!("/proc/{}/fd", p.pid))
                                .ok()
                                .map(|entries| entries.filter_map(Result::ok).count());
                            values.insert("fd_count".into(), json!(count));
                        }
                        values.insert("details_sampled_at".into(), json!(unix_time()));
                        values.insert("delay_accounting_status".into(),json!("CPU/swap delays require taskstats permissions; uncollected values stay unavailable"));
                        self.process_fields.insert(identity, (now, values));
                    }
                    if let Some((_, values)) = self.process_fields.get(&identity) {
                        p.extra.extend(values.clone());
                    }
                }
                if delay_requested && delay_began.elapsed() < Duration::from_millis(100) {
                    match self.delays.collect(p.pid, p.start_time, p.thread) {
                        Ok(value) => {
                            if let Some(fields) = value.as_object() {
                                p.extra
                                    .extend(fields.iter().map(|(k, v)| (k.clone(), v.clone())));
                            }
                        }
                        Err(error) => {
                            p.extra
                                .insert("delay_accounting_status".into(), json!(error.to_string()));
                        }
                    }
                }
                if (stat_requested || p.nice.is_none())
                    && let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", p.pid))
                    && let Some(tail) = stat.rsplit_once(')')
                {
                    let f: Vec<_> = tail.1.split_whitespace().collect();
                    if stat_requested {
                        p.extra
                            .insert("stat".into(), json!(parse_process_stat(&stat)));
                    }
                    p.nice = f.get(16).and_then(|v| v.parse().ok());
                    p.kernel_thread = f
                        .get(6)
                        .and_then(|v| v.parse::<u64>().ok())
                        .is_some_and(|flags| flags & 0x200000 != 0);
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (s, config);
        }
    }
    #[cfg(target_os = "linux")]
    fn drm_processes(&mut self, s: &mut Snapshot, now: Instant) {
        let cards: HashMap<String, u32> = s
            .gpus
            .iter()
            .filter(|g| g.backend.contains("sysfs"))
            .map(|g| (g.uuid.clone(), g.id))
            .collect();
        if cards.is_empty() {
            return;
        }
        let drm_fds = crate::connections::drm_fds();
        let mut next = HashMap::new();
        for process in &mut s.processes {
            let mut seen = HashSet::new();
            let mut engines = BTreeMap::new();
            let mut gpu_memory: HashMap<u32, u64> = HashMap::new();
            let mut per_gpu_util = HashMap::<u32, f64>::new();
            let mut clients = vec![];
            let mut ids = HashSet::new();
            for path in drm_fds.get(&process.pid).into_iter().flatten() {
                let Ok(text) = fs::read_to_string(path) else {
                    continue;
                };
                let fields = parse_fdinfo(&text);
                let Some(client) = fields.get("drm-client-id") else {
                    continue;
                };
                let Some(pci) = fields.get("drm-pdev") else {
                    continue;
                };
                let Some(&id) = cards.get(pci) else {
                    continue;
                };
                if !seen.insert((pci.clone(), client.clone())) {
                    continue;
                }
                ids.insert(id);
                clients.push(json!({"gpu_id":id,"fields":fields}));
                for (key, value) in &fields {
                    if key.starts_with("drm-engine-") && !key.starts_with("drm-engine-capacity-") {
                        let Some(ns) = value
                            .split_whitespace()
                            .next()
                            .and_then(|v| v.parse::<u64>().ok())
                        else {
                            continue;
                        };
                        let identity = (
                            process.pid,
                            format!("{}:{pci}:{client}", process.start_time),
                            key.clone(),
                        );
                        if let Some((old, at)) = self.drm.get(&identity) {
                            let util =
                                rate(Some(*old), ns, now.duration_since(*at).as_secs_f64()) / 1e7;
                            let capacity = fields
                                .get(&key.replace("drm-engine-", "drm-engine-capacity-"))
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(1.0)
                                .max(1.0);
                            let util = (util / capacity).clamp(0.0, 100.0);
                            let key = format!("GPU{id} {}", key.trim_start_matches("drm-engine-"));
                            let sum =
                                engines.get(&key).and_then(Value::as_f64).unwrap_or(0.0) + util;
                            engines.insert(key, json!(sum));
                        }
                        let total = process
                            .extra
                            .get("drm_engine_time_ns")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        process
                            .extra
                            .insert("drm_engine_time_ns".into(), json!(total.saturating_add(ns)));
                        next.insert(identity, (ns, now));
                    }
                }
                // resident-vram is preferable; memory-vram is an older fdinfo alias.
                if let Some(value) = fields
                    .get("drm-resident-vram")
                    .or_else(|| fields.get("drm-memory-vram"))
                    && let Some(bytes) = crate::discovery::parse_size(value)
                {
                    *gpu_memory.entry(id).or_default() += bytes;
                }
                if !process.gpu_ids.contains(&id) {
                    process.gpu_ids.push(id);
                }
            }
            for id in &ids {
                let prefix = format!("GPU{id} ");
                let maximum = engines
                    .iter()
                    .filter(|(key, _)| key.starts_with(&prefix))
                    .filter_map(|(_, v)| v.as_f64())
                    .reduce(f64::max);
                if let Some(value) = maximum {
                    per_gpu_util.insert(*id, value.clamp(0.0, 100.0));
                }
            }
            if !engines.is_empty() {
                let utilization = per_gpu_util.values().sum::<f64>() as u32;
                process.gpu_percent =
                    Some(process.gpu_percent.unwrap_or(0).saturating_add(utilization));
                process
                    .extra
                    .insert("DRM engines (%)".into(), json!(engines));
            }
            if !clients.is_empty() {
                process.extra.insert(
                    "DRM clients (VRAM and shared memory counters)".into(),
                    json!(clients),
                );
            }
            if !gpu_memory.is_empty() {
                process.gpu_memory = Some(
                    process
                        .gpu_memory
                        .unwrap_or(0)
                        .saturating_add(gpu_memory.values().sum::<u64>()),
                );
            }
            for id in ids {
                if let Some(gpu) = s.gpus.iter_mut().find(|g| g.id == id) {
                    gpu.processes.push(GpuProcess {
                        pid: process.pid,
                        memory: gpu_memory.get(&id).copied(),
                        utilization: per_gpu_util.get(&id).map(|v| *v as u32),
                        kind: "DRM".into(),
                        ..Default::default()
                    });
                    gpu.process_status = "DRM fdinfo (readable processes)".into();
                }
            }
        }
        self.drm = next;
    }
}
pub fn parse_fdinfo(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (k, v) = line.split_once(':')?;
            Some((k.trim().into(), v.trim().into()))
        })
        .collect()
}
pub fn parse_full_diskstats(text: &str) -> HashMap<String, Vec<u64>> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() < 14 {
                return None;
            }
            let values: Vec<_> = f[3..]
                .iter()
                .map(|v| v.parse::<u64>())
                .collect::<Result<_, _>>()
                .ok()?;
            Some((f[2].to_string(), values))
        })
        .collect()
}
#[cfg(target_os = "linux")]
fn read(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().into())
}
#[cfg(target_os = "linux")]
fn number(path: impl AsRef<Path>) -> Option<u64> {
    read(path)?.parse().ok()
}

pub fn parse_process_fields(text: &str) -> BTreeMap<String, Value> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let mut words = value.split_whitespace();
            let first = words.next().unwrap_or_default();
            let second = words.next();
            let value = if key.starts_with("Cap") || key.starts_with("Sig") {
                json!(value.trim())
            } else if second == Some("kB") {
                first
                    .parse::<u64>()
                    .ok()
                    .and_then(|v| v.checked_mul(1024))
                    .map(Value::from)
                    .unwrap_or(Value::Null)
            } else if second.is_none() {
                first
                    .parse::<u64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| json!(value.trim()))
            } else {
                json!(value.trim())
            };
            Some((key.into(), value))
        })
        .collect()
}

/// Linux /proc/PID/stat: locate the final ')' because comm may contain spaces or parentheses.
pub fn parse_process_stat(text: &str) -> BTreeMap<String, Value> {
    let Some((_, tail)) = text.rsplit_once(')') else {
        return BTreeMap::new();
    };
    let fields: Vec<_> = tail.split_whitespace().collect();
    [
        ("pgrp", 2),
        ("session", 3),
        ("tty", 4),
        ("tpgid", 5),
        ("minor_faults", 7),
        ("minor_faults_children", 8),
        ("major_faults", 9),
        ("major_faults_children", 10),
        ("utime_ticks", 11),
        ("stime_ticks", 12),
        ("children_utime_ticks", 13),
        ("children_stime_ticks", 14),
        ("priority", 15),
        ("threads", 17),
        ("processor", 36),
        ("rt_priority", 37),
        ("scheduling", 38),
    ]
    .into_iter()
    .filter_map(|(key, i)| Some((key.into(), Value::from(fields.get(i)?.parse::<i64>().ok()?))))
    .collect()
}
