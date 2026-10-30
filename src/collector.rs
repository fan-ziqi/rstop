use crate::{config::Config, gpu::GpuCollector, model::*};
use std::{
    collections::HashMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{
    Components, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind, Users,
};

pub fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Cheap first-frame data, before driver initialization, disk scans, or CPU rate sampling.
pub fn initial_snapshot() -> Snapshot {
    let mut system = System::new();
    system.refresh_cpu_usage();
    system.refresh_memory();
    let load = System::load_average();
    Snapshot {
        hostname: System::host_name().unwrap_or_else(|| "localhost".into()),
        uptime: System::uptime(),
        cpu_name: system
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default(),
        cores: vec![0.; system.cpus().len()],
        load: [load.one, load.five, load.fifteen],
        memory: Memory {
            total: system.total_memory(),
            used: system.used_memory(),
            available: system.available_memory(),
            free: Some(system.free_memory()),
            cached: cached_memory(),
            swap_total: system.total_swap(),
            swap_used: system.used_swap(),
        },
        ..Default::default()
    }
}

const STORAGE_REFRESH: std::time::Duration = std::time::Duration::from_secs(10);
const SENSOR_REFRESH: std::time::Duration = std::time::Duration::from_secs(2);

pub struct Collector {
    system: System,
    networks: Networks,
    disks: Disks,
    components: Components,
    components_at: Instant,
    disks_at: Instant,
    users: Users,
    gpu: GpuCollector,
    previous: Instant,
    boot: Instant,
    network_counters: HashMap<String, (u64, u64, u64, u64)>,
    disk_counters: HashMap<String, (u64, u64, u64)>,
    cycles: u64,
    cpu_counters: Vec<u64>,
    core_counters: Vec<Vec<u64>>,
    advanced: crate::advanced::Advanced,
    physical_cores: Option<usize>,
    core_topology: Vec<Option<(u32, u32)>>,
    #[cfg(target_os = "linux")]
    processes: crate::procfs::Processes,
}

impl Collector {
    pub fn new(gpu: bool) -> Self {
        let mut system = System::new();
        system.refresh_cpu_all();
        system.refresh_memory();
        #[cfg(not(target_os = "linux"))]
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, process_refresh(false));
        let networks = Networks::new_with_refreshed_list();
        let network_counters = networks
            .iter()
            .map(|(name, n)| {
                (
                    name.clone(),
                    (
                        n.total_received(),
                        n.total_transmitted(),
                        n.total_packets_received(),
                        n.total_packets_transmitted(),
                    ),
                )
            })
            .collect();
        let now = Instant::now();
        let core_topology = cpu_topology(&system);
        let (cpu_counters, core_counters) = read_cpu_counters();
        Self {
            system,
            networks,
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            components_at: now,
            disks_at: now,
            users: Users::new_with_refreshed_list(),
            gpu: GpuCollector::new(gpu),
            previous: now,
            boot: now,
            network_counters,
            disk_counters: read_disk_counters(),
            cycles: 0,
            cpu_counters,
            core_counters,
            advanced: Default::default(),
            physical_cores: System::physical_core_count(),
            core_topology,
            #[cfg(target_os = "linux")]
            processes: Default::default(),
        }
    }

    pub fn collect(&mut self, config: &Config) -> Snapshot {
        self.collect_with_processes(config, true)
    }

    pub fn collect_with_processes(
        &mut self,
        config: &Config,
        live_gpu_processes: bool,
    ) -> Snapshot {
        self.collect_selected(config, true, live_gpu_processes)
    }

    pub fn collect_selected(
        &mut self,
        config: &Config,
        processes_enabled: bool,
        live_gpu_processes: bool,
    ) -> Snapshot {
        let began = Instant::now();
        let elapsed = began.duration_since(self.previous).as_secs_f64().max(0.001);
        self.previous = began;
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        if processes_enabled && (cfg!(not(target_os = "linux")) || config.show_threads) {
            self.system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                process_refresh(config.show_threads),
            );
        }
        self.networks.refresh(true);
        // Reading every hardware sensor costs far more than the rest of a sample put together
        // on macOS, and temperatures move slowly, so refresh them at most every two seconds.
        if began.duration_since(self.components_at) >= SENSOR_REFRESH {
            self.components.refresh(true);
            self.components_at = began;
        }
        // Capacity and mount changes cost ~16 ms per refresh on macOS and move slowly.
        if began.duration_since(self.disks_at) >= STORAGE_REFRESH {
            self.disks.refresh_specifics(
                true,
                sysinfo::DiskRefreshKind::nothing()
                    .with_storage()
                    .with_kind(),
            );
            self.disks_at = began;
        }
        if self.cycles.is_multiple_of(5) {
            self.system.refresh_cpu_frequency();
        }
        #[cfg(not(target_os = "linux"))]
        self.disks
            .refresh_specifics(true, sysinfo::DiskRefreshKind::nothing().with_io_usage());
        if self.cycles.is_multiple_of(60) {
            self.users.refresh();
        }
        self.cycles += 1;
        if self.core_topology.len() != self.system.cpus().len() {
            self.core_topology = cpu_topology(&self.system);
        }
        let (gpus, gpu_status) = self.gpu.collect_with_processes(live_gpu_processes);
        type GpuAttribution = (Option<u64>, Option<u32>, Vec<u32>);
        let mut gpu_processes: HashMap<u32, GpuAttribution> = HashMap::new();
        for gpu in &gpus {
            if gpu
                .extra
                .get("mig_children")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
            {
                continue;
            }
            for process in &gpu.processes {
                let entry = gpu_processes
                    .entry(process.pid)
                    .or_insert((None, None, vec![]));
                if let Some(memory) = process.memory {
                    entry.0 = Some(entry.0.unwrap_or(0) + memory);
                }
                if let Some(utilization) = process.utilization {
                    entry.1 = Some(entry.1.unwrap_or(0) + utilization);
                }
                entry.2.push(gpu.id);
            }
        }
        let mut processes = vec![];
        #[cfg(target_os = "linux")]
        if processes_enabled && !config.show_threads {
            processes = self.processes.collect(&self.users, elapsed);
            for p in &mut processes {
                if let Some(gpu) = gpu_processes.get(&p.pid) {
                    p.gpu_memory = gpu.0;
                    p.gpu_percent = gpu.1;
                    p.gpu_ids = gpu.2.clone();
                }
            }
        }
        if processes_enabled && (cfg!(not(target_os = "linux")) || config.show_threads) {
            for (pid, p) in self.system.processes() {
                let io = p.disk_usage();
                let gpu = gpu_processes.get(&pid.as_u32());
                let command = p
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");
                let user = p
                    .user_id()
                    .and_then(|id| self.users.get_user_by_id(id))
                    .map(|u| u.name().to_string())
                    .unwrap_or_else(|| {
                        p.user_id()
                            .map(|id| format!("{id:?}"))
                            .unwrap_or_else(|| "—".into())
                    });
                processes.push(Process {
                    pid: pid.as_u32(),
                    parent: p.parent().map(|id| id.as_u32()),
                    start_time: p.start_time(),
                    name: p.name().to_string_lossy().into_owned(),
                    command: command.chars().filter(|c| !c.is_control()).collect(),
                    user,
                    state: p.status().to_string(),
                    cpu: p.cpu_usage(),
                    memory: p.memory(),
                    virtual_memory: p.virtual_memory(),
                    cpu_time_ms: p.accumulated_cpu_time(),
                    read_per_sec: io.read_bytes as f64 / elapsed,
                    write_per_sec: io.written_bytes as f64 / elapsed,
                    read_total: io.total_read_bytes,
                    write_total: io.total_written_bytes,
                    gpu_memory: gpu.and_then(|p| p.0),
                    gpu_percent: gpu.and_then(|p| p.1),
                    gpu_ids: gpu.map(|p| p.2.clone()).unwrap_or_default(),
                    thread: matches!(p.thread_kind(), Some(sysinfo::ThreadKind::Userland)),
                    runtime: p.run_time(),
                    ..Process::default()
                });
            }
        }
        processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(a.pid.cmp(&b.pid)));
        let mut networks = vec![];
        let mut current_networks = HashMap::new();
        for (name, data) in &self.networks {
            let rx = data.total_received();
            let tx = data.total_transmitted();
            let previous = self.network_counters.get(name);
            networks.push(Network {
                name: name.clone(),
                mac: data.mac_address().to_string(),
                mtu: data.mtu(),
                rx_packets: data.total_packets_received(),
                tx_packets: data.total_packets_transmitted(),
                rx_packets_per_sec: previous
                    .filter(|p| data.total_packets_received() >= p.2)
                    .map(|p| rate(Some(p.2), data.total_packets_received(), elapsed)),
                tx_packets_per_sec: previous
                    .filter(|p| data.total_packets_transmitted() >= p.3)
                    .map(|p| rate(Some(p.3), data.total_packets_transmitted(), elapsed)),
                rx_errors: data.total_errors_on_received(),
                tx_errors: data.total_errors_on_transmitted(),
                speed_mbps: {
                    #[cfg(target_os = "linux")]
                    {
                        std::fs::read_to_string(format!("/sys/class/net/{name}/speed"))
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok())
                            .filter(|v| *v > 0)
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        None
                    }
                },
                rx_per_sec: rate(previous.map(|n| n.0), rx, elapsed),
                tx_per_sec: rate(previous.map(|n| n.1), tx, elapsed),
                rx_total: rx,
                tx_total: tx,
                errors: data.total_errors_on_received() + data.total_errors_on_transmitted(),
                drops: network_drops(name),
                address: data
                    .ip_networks()
                    .iter()
                    .map(|n| n.addr.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                state: network_state(name),
            });
            current_networks.insert(
                name.clone(),
                (
                    rx,
                    tx,
                    data.total_packets_received(),
                    data.total_packets_transmitted(),
                ),
            );
        }
        self.network_counters = current_networks;
        networks.sort_by(|a, b| {
            b.rx_per_sec
                .total_cmp(&a.rx_per_sec)
                .then(a.name.cmp(&b.name))
        });
        let disks =
            self.disks
                .iter()
                .filter(|d| d.total_space() > 0)
                // The sealed read-only system volume shares its APFS container with the Data volume.
                .filter(|d| {
                    !(cfg!(target_os = "macos")
                        && d.mount_point() == std::path::Path::new("/")
                        && self.disks.iter().any(|o| {
                            o.mount_point() == std::path::Path::new("/System/Volumes/Data")
                        }))
                })
                .map(|d| Disk {
                    name: d.name().to_string_lossy().into_owned(),
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    filesystem: d.file_system().to_string_lossy().into_owned(),
                    kind: format!("{:?}", d.kind()),
                    removable: d.is_removable(),
                    available: d.available_space(),
                    extra: filesystem_fields(d.mount_point()),
                    total: d.total_space(),
                    used: d.total_space().saturating_sub(d.available_space()),
                })
                .collect();
        let counters = read_disk_counters();
        let mut disk_io: Vec<_> = counters
            .iter()
            .map(|(name, &(read, write, busy))| {
                let prev = self.disk_counters.get(name);
                DiskIo {
                    name: name.clone(),
                    read_per_sec: rate(prev.map(|v| v.0), read, elapsed),
                    write_per_sec: rate(prev.map(|v| v.1), write, elapsed),
                    busy_percent: Some(
                        (rate(prev.map(|v| v.2), busy, elapsed) / 10.0).clamp(0.0, 100.0),
                    ),
                    aggregate: std::path::Path::new(&format!("/sys/class/block/{name}/device"))
                        .exists()
                        && !std::path::Path::new(&format!("/sys/class/block/{name}/partition"))
                            .exists(),
                    ..DiskIo::default()
                }
            })
            .collect();
        disk_io.sort_by(|a, b| a.name.cmp(&b.name));
        #[cfg(not(target_os = "linux"))]
        {
            let mut seen = std::collections::HashSet::new();
            for disk in &self.disks {
                let name = disk.name().to_string_lossy().into_owned();
                if !seen.insert(name.clone()) {
                    continue;
                }
                let usage = disk.usage();
                disk_io.push(DiskIo {
                    name,
                    read_per_sec: usage.read_bytes as f64 / elapsed,
                    write_per_sec: usage.written_bytes as f64 / elapsed,
                    busy_percent: None,
                    aggregate: true,
                    ..DiskIo::default()
                });
            }
        }
        self.disk_counters = counters;
        let memory = Memory {
            total: self.system.total_memory(),
            used: self.system.used_memory(),
            available: self.system.available_memory(),
            free: Some(self.system.free_memory()),
            cached: cached_memory(),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap(),
        };
        let load = System::load_average();
        let sensors = self
            .components
            .iter()
            .filter_map(|c| {
                c.temperature().map(|temperature| Sensor {
                    name: c.label().to_string(),
                    temperature,
                    critical: c.critical(),
                })
            })
            .collect();
        let (cpu_counters, core_counters) = read_cpu_counters();
        let cpu_breakdown = cpu_breakdown(&self.cpu_counters, &cpu_counters);
        let core_times = core_times(
            &self.core_counters,
            &core_counters,
            self.system.cpus().len(),
        );
        self.cpu_counters = cpu_counters;
        self.core_counters = core_counters;
        let mut snapshot = Snapshot {
            timestamp: unix_time(),
            elapsed: self.boot.elapsed().as_secs_f64(),
            collection_ms: 0.0,
            hostname: System::host_name().unwrap_or_else(|| "localhost".into()),
            os: System::long_os_version().unwrap_or_default(),
            kernel: System::kernel_version().unwrap_or_default(),
            uptime: System::uptime(),
            cpu_name: self
                .system
                .cpus()
                .first()
                .map(|c| c.brand().trim().to_string())
                .unwrap_or_default(),
            cpu_percent: self.system.global_cpu_usage(),
            // A single sleeping core can report the minimum clock while other
            // cores are boosting. Match the overall CPU readout to the mean of
            // the readable logical-core frequencies.
            cpu_mhz: {
                let (total, count) = self
                    .system
                    .cpus()
                    .iter()
                    .map(|cpu| cpu.frequency())
                    .filter(|mhz| *mhz > 0)
                    .fold((0u64, 0u64), |(total, count), mhz| (total + mhz, count + 1));
                total.checked_div(count).unwrap_or(0)
            },
            cpu_breakdown,
            cores: self.system.cpus().iter().map(|c| c.cpu_usage()).collect(),
            core_times,
            core_temperatures: core_temperatures(&self.core_topology),
            load: [load.one, load.five, load.fifteen],
            memory,
            processes,
            networks,
            connections: Default::default(),
            disks,
            disk_io,
            sensors,
            batteries: batteries(),
            gpus,
            gpu_status,
            pressure: pressure(),
            alerts: vec![],
            inventory: Inventory::default(),
            extra: Default::default(),
        };
        if snapshot.cpu_percent >= config.cpu_warning {
            snapshot
                .alerts
                .push(format!("CPU {:.0}%", snapshot.cpu_percent));
        }
        let memory_percent = percent(snapshot.memory.used, snapshot.memory.total);
        if memory_percent >= config.memory_warning {
            snapshot.alerts.push(format!("Memory {memory_percent:.0}%"));
        }
        for disk in &snapshot.disks {
            if percent(disk.used, disk.total) >= 90.0 {
                snapshot.alerts.push(format!("Disk {} ≥90%", disk.mount));
            }
        }
        for gpu in &snapshot.gpus {
            if gpu
                .temperature
                .is_some_and(|t| t >= config.gpu_temperature_warning)
            {
                snapshot.alerts.push(format!(
                    "GPU {} hot: {}°C",
                    gpu.id,
                    gpu.temperature.unwrap()
                ));
            }
        }
        for p in &snapshot.pressure {
            if p.some_10.is_some_and(|v| v > 20.0) {
                snapshot.alerts.push(format!(
                    "{} pressure {:.0}%",
                    p.resource,
                    p.some_10.unwrap()
                ));
            }
        }
        self.advanced.collect(&mut snapshot, config);
        snapshot.extra.insert(
            "Collection state".into(),
            serde_json::json!({"processes":processes_enabled,"gpu_processes":live_gpu_processes}),
        );
        snapshot.extra.insert("CPU topology".into(),serde_json::json!({"architecture":std::env::consts::ARCH,"logical_cores":snapshot.cores.len(),"physical_cores":self.physical_cores,"per_core_frequency_mhz":self.system.cpus().iter().map(|cpu|cpu.frequency()).collect::<Vec<_>>() }));
        let mut states = std::collections::BTreeMap::<String, u64>::new();
        let mut threads = None::<u64>;
        for p in &snapshot.processes {
            if !p.thread {
                *states.entry(p.state.clone()).or_default() += 1;
                if let Some(n) = p
                    .extra
                    .get("stat")
                    .and_then(|v| v.get("threads"))
                    .and_then(|v| v.as_u64())
                {
                    threads = Some(threads.unwrap_or(0) + n);
                }
            }
        }
        snapshot.extra.insert("Process counts".into(),serde_json::json!({"states":states,"threads":threads,"sampled_rows":snapshot.processes.len()}));
        snapshot.collection_ms = began.elapsed().as_secs_f64() * 1000.0;
        snapshot
    }
}

fn process_refresh(threads: bool) -> ProcessRefreshKind {
    let kind = ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
        .without_tasks();
    if threads { kind.with_tasks() } else { kind }
}

pub fn rate(previous: Option<u64>, current: u64, seconds: f64) -> f64 {
    match previous {
        Some(old) if current >= old && seconds > 0.0 => (current - old) as f64 / seconds,
        _ => 0.0,
    }
}

fn read_cpu_counters() -> (Vec<u64>, Vec<Vec<u64>>) {
    #[cfg(target_os = "linux")]
    {
        parse_proc_stat(&std::fs::read_to_string("/proc/stat").unwrap_or_default())
    }
    #[cfg(target_os = "macos")]
    {
        let mut info = unsafe { std::mem::zeroed::<libc::host_cpu_load_info>() };
        let mut count = libc::HOST_CPU_LOAD_INFO_COUNT;
        #[allow(deprecated)]
        let result = unsafe {
            libc::host_statistics(
                libc::mach_host_self(),
                libc::HOST_CPU_LOAD_INFO,
                (&mut info as *mut libc::host_cpu_load_info).cast(),
                &mut count,
            )
        };
        if result != 0 {
            return (vec![], vec![]);
        }
        (
            vec![
                info.cpu_ticks[0] as u64,
                info.cpu_ticks[3] as u64,
                info.cpu_ticks[1] as u64,
                info.cpu_ticks[2] as u64,
            ],
            Vec::new(),
        )
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
        let mut idle = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = idle;
        let mut user = idle;
        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            return (vec![], vec![]);
        }
        let value = |v: FILETIME| ((v.dwHighDateTime as u64) << 32) | v.dwLowDateTime as u64;
        (
            vec![
                value(user),
                value(kernel).saturating_sub(value(idle)),
                value(idle),
            ],
            Vec::new(),
        )
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        (vec![], vec![])
    }
}

/// Split `/proc/stat` CPU time counters into the aggregate and per-core rows, in file order.
pub fn parse_proc_stat(text: &str) -> (Vec<u64>, Vec<Vec<u64>>) {
    let mut aggregate = Vec::new();
    let mut cores = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(label) = fields.next() else {
            continue;
        };
        if label == "cpu" {
            aggregate = fields.take(10).filter_map(|v| v.parse().ok()).collect();
            continue;
        }
        let Some(index) = label.strip_prefix("cpu") else {
            continue;
        };
        if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let row = fields
            .take(10)
            .map(|value| value.parse::<u64>())
            .collect::<Result<Vec<_>, _>>();
        if let Ok(row) = row
            && matches!(row.len(), 8 | 10)
        {
            cores.push(row);
        }
    }
    (aggregate, cores)
}

/// Per-core time shares from two samples; empty unless both are complete and aligned.
pub fn core_times(previous: &[Vec<u64>], current: &[Vec<u64>], expected: usize) -> Vec<CoreTime> {
    if previous.len() != current.len() || current.len() != expected || current.is_empty() {
        return Vec::new();
    }
    previous
        .iter()
        .zip(current)
        .map(|(previous, current)| CoreTime::from_breakdown(&cpu_breakdown(previous, current)))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

pub fn cpu_breakdown(previous: &[u64], current: &[u64]) -> std::collections::BTreeMap<String, f64> {
    if ![3, 4, 8, 10].contains(&previous.len())
        || current.len() != previous.len()
        || previous.iter().zip(current).any(|(a, b)| b < a)
    {
        return Default::default();
    }
    let mut delta: Vec<_> = current.iter().zip(previous).map(|(b, a)| b - a).collect();
    let total: u64 = delta.iter().take(8).sum();
    if delta.len() == 10 {
        delta[0] = delta[0].saturating_sub(delta[8]);
        delta[1] = delta[1].saturating_sub(delta[9]);
    }
    if total == 0 {
        return Default::default();
    }
    let labels: &[&str] = match current.len() {
        3 => &["user", "system", "idle"],
        4 => &["user", "nice", "system", "idle"],
        10 => &[
            "user",
            "nice",
            "system",
            "idle",
            "iowait",
            "irq",
            "softirq",
            "steal",
            "guest",
            "guest_nice",
        ],
        _ => &[
            "user", "nice", "system", "idle", "iowait", "irq", "softirq", "steal",
        ],
    };
    labels
        .iter()
        .copied()
        .zip(delta)
        .map(|(name, value)| (name.to_string(), value as f64 / total as f64 * 100.0))
        .collect()
}

#[cfg(target_os = "linux")]
fn read_disk_counters() -> HashMap<String, (u64, u64, u64)> {
    parse_diskstats(&std::fs::read_to_string("/proc/diskstats").unwrap_or_default())
}
#[cfg(not(target_os = "linux"))]
fn read_disk_counters() -> HashMap<String, (u64, u64, u64)> {
    HashMap::new()
}

pub fn parse_diskstats(text: &str) -> HashMap<String, (u64, u64, u64)> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() < 14 || f[2].starts_with("loop") || f[2].starts_with("ram") {
                return None;
            }
            // Block layer sectors are always 512 bytes, irrespective of physical sector size.
            Some((
                f[2].to_string(),
                (
                    f[5].parse::<u64>().ok()? * 512,
                    f[9].parse::<u64>().ok()? * 512,
                    f[12].parse().ok()?,
                ),
            ))
        })
        .collect()
}

fn cached_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        Some(
            std::fs::read_to_string("/proc/meminfo")
                .ok()?
                .lines()
                .filter(|l| l.starts_with("Cached:") || l.starts_with("SReclaimable:"))
                .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                .sum::<u64>()
                * 1024,
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn network_state(_name: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string(format!("/sys/class/net/{_name}/operstate"))
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string()
    }
    #[cfg(not(target_os = "linux"))]
    {
        "—".into()
    }
}

fn network_drops(_name: &str) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        Some(
            ["rx_dropped", "tx_dropped"]
                .iter()
                .filter_map(|field| {
                    std::fs::read_to_string(format!("/sys/class/net/{_name}/statistics/{field}"))
                        .ok()?
                        .trim()
                        .parse::<u64>()
                        .ok()
                })
                .sum(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn pressure() -> Vec<Pressure> {
    #[cfg(target_os = "linux")]
    {
        ["cpu", "memory", "io"]
            .iter()
            .filter_map(|name| {
                std::fs::read_to_string(format!("/proc/pressure/{name}"))
                    .ok()
                    .map(|text| parse_pressure(name, &text))
            })
            .collect()
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec![]
    }
}

pub fn parse_pressure(name: &str, text: &str) -> Pressure {
    let value = |kind: &str, key: &str| {
        text.lines()
            .find(|l| l.starts_with(kind))?
            .split_whitespace()
            .find_map(|f| f.strip_prefix(key))?
            .parse::<f64>()
            .ok()
    };
    Pressure {
        resource: name.to_string(),
        some_10: value("some ", "avg10="),
        some_60: value("some ", "avg60="),
        some_300: value("some ", "avg300="),
        some_total_us: value("some ", "total=").map(|v| v as u64),
        full_10: value("full ", "avg10="),
        full_60: value("full ", "avg60="),
        full_300: value("full ", "avg300="),
        full_total_us: value("full ", "total=").map(|v| v as u64),
    }
}

fn batteries() -> Vec<Battery> {
    use starship_battery::units::{
        electric_potential::volt, energy::watt_hour, power::watt, ratio::percent,
        thermodynamic_temperature::degree_celsius, time::second,
    };
    let Ok(manager) = starship_battery::Manager::new() else {
        return vec![];
    };
    let Ok(items) = manager.batteries() else {
        return vec![];
    };
    items
        .enumerate()
        .filter_map(|(index, item)| {
            let battery = item.ok()?;
            Some(Battery {
                extra: std::collections::BTreeMap::from([
                    (
                        "health_percent".into(),
                        serde_json::json!(battery.state_of_health().get::<percent>()),
                    ),
                    (
                        "energy_wh".into(),
                        serde_json::json!(battery.energy().get::<watt_hour>()),
                    ),
                    (
                        "full_capacity_wh".into(),
                        serde_json::json!(battery.energy_full().get::<watt_hour>()),
                    ),
                    (
                        "design_capacity_wh".into(),
                        serde_json::json!(battery.energy_full_design().get::<watt_hour>()),
                    ),
                    (
                        "voltage_v".into(),
                        serde_json::json!(battery.voltage().get::<volt>()),
                    ),
                    (
                        "temperature_celsius".into(),
                        serde_json::json!(battery.temperature().map(|v| v.get::<degree_celsius>())),
                    ),
                    (
                        "time_to_empty_seconds".into(),
                        serde_json::json!(battery.time_to_empty().map(|v| v.get::<second>())),
                    ),
                    (
                        "time_to_full_seconds".into(),
                        serde_json::json!(battery.time_to_full().map(|v| v.get::<second>())),
                    ),
                    (
                        "cycle_count".into(),
                        serde_json::json!(battery.cycle_count()),
                    ),
                    ("vendor".into(), serde_json::json!(battery.vendor())),
                    (
                        "technology".into(),
                        serde_json::json!(format!("{:?}", battery.technology())),
                    ),
                ]),
                name: battery
                    .model()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("Battery {index}")),
                percent: Some(battery.state_of_charge().get::<percent>()),
                status: format!("{:?}", battery.state()),
                watts: Some(battery.energy_rate().get::<watt>()),
            })
        })
        .collect()
}

fn filesystem_fields(
    path: &std::path::Path,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut fields = std::collections::BTreeMap::new();
        if let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) {
            let mut stat = unsafe { std::mem::zeroed::<libc::statvfs>() };
            if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } == 0 {
                fields.insert("inodes_total".into(), serde_json::json!(stat.f_files));
                fields.insert("inodes_free".into(), serde_json::json!(stat.f_ffree));
                fields.insert("block_size_bytes".into(), serde_json::json!(stat.f_frsize));
                fields.insert(
                    "readonly".into(),
                    serde_json::json!(stat.f_flag & libc::ST_RDONLY != 0),
                );
            }
        }
        fields
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Default::default()
    }
}

fn cpu_topology(system: &System) -> Vec<Option<(u32, u32)>> {
    system
        .cpus()
        .iter()
        .map(|cpu| {
            #[cfg(target_os = "linux")]
            {
                let id = cpu.name().strip_prefix("cpu")?.parse::<u32>().ok()?;
                let base =
                    std::path::PathBuf::from(format!("/sys/devices/system/cpu/cpu{id}/topology"));
                let number = |name| {
                    std::fs::read_to_string(base.join(name))
                        .ok()?
                        .trim()
                        .parse::<u32>()
                        .ok()
                };
                Some((number("physical_package_id")?, number("core_id")?))
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = cpu;
                None
            }
        })
        .collect()
}
fn core_temperatures(topology: &[Option<(u32, u32)>]) -> Vec<Option<f32>> {
    core_temperatures_at(std::path::Path::new("/sys/class/hwmon"), topology)
}
fn core_temperatures_at(
    root: &std::path::Path,
    topology: &[Option<(u32, u32)>],
) -> Vec<Option<f32>> {
    #[cfg(target_os = "linux")]
    {
        let mut temperatures = std::collections::HashMap::new();
        for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let path = entry.path();
            if std::fs::read_to_string(path.join("name"))
                .ok()
                .is_none_or(|name| name.trim() != "coretemp")
            {
                continue;
            }
            let labels: Vec<_> = std::fs::read_dir(&path)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    (name.starts_with("temp") && name.ends_with("_label"))
                        .then(|| {
                            Some((
                                name,
                                std::fs::read_to_string(e.path()).ok()?.trim().to_string(),
                            ))
                        })
                        .flatten()
                })
                .collect();
            let package = labels
                .iter()
                .find_map(|(_, label)| label.strip_prefix("Package id ")?.parse::<u32>().ok());
            let Some(package) = package else {
                continue;
            };
            for (name, label) in labels {
                let Some(core) = label
                    .strip_prefix("Core ")
                    .and_then(|s| s.parse::<u32>().ok())
                else {
                    continue;
                };
                if let Some(temperature) =
                    std::fs::read_to_string(path.join(name.replace("_label", "_input")))
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok())
                        .filter(|v| v.is_finite())
                {
                    temperatures.insert((package, core), temperature / 1000.);
                }
            }
        }
        topology
            .iter()
            .map(|key| key.and_then(|key| temperatures.get(&key).copied()))
            .collect()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = root;
        vec![None; topology.len()]
    }
}

#[cfg(all(test, target_os = "linux"))]
mod core_sensor_tests {
    #[test]
    fn physical_package_and_core_ids_prevent_cross_socket_temperature_aliases() {
        let root = std::env::temp_dir().join(format!(
            "rstop-core-sensors-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for (name, driver, package, input) in [
            ("hwmon0", "coretemp", 0, 46000),
            ("hwmon1", "coretemp", 1, 53000),
            ("hwmon2", "nvme", 0, 99000),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(&path).unwrap();
            for (file, value) in [
                ("name", driver.to_string()),
                ("temp1_label", format!("Package id {package}")),
                ("temp2_label", "Core 0".into()),
                ("temp2_input", input.to_string()),
            ] {
                std::fs::write(path.join(file), value).unwrap();
            }
        }
        let actual = super::core_temperatures_at(
            &root,
            &[Some((0, 0)), Some((1, 0)), Some((0, 0)), Some((0, 1)), None],
        );
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(actual, vec![Some(46.), Some(53.), Some(46.), None, None]);
    }
}
