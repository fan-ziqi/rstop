use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub timestamp: u64,
    pub elapsed: f64,
    pub collection_ms: f64,
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub uptime: u64,
    pub cpu_name: String,
    pub cpu_percent: f32,
    pub cpu_mhz: u64,
    pub cpu_breakdown: std::collections::BTreeMap<String, f64>,
    pub cores: Vec<f32>,
    pub core_temperatures: Vec<Option<f32>>,
    pub load: [f64; 3],
    pub memory: Memory,
    pub processes: Vec<Process>,
    pub networks: Vec<Network>,
    pub connections: crate::connections::Sample,
    pub disks: Vec<Disk>,
    pub disk_io: Vec<DiskIo>,
    pub sensors: Vec<Sensor>,
    pub batteries: Vec<Battery>,
    pub gpus: Vec<Gpu>,
    pub gpu_status: String,
    pub pressure: Vec<Pressure>,
    pub alerts: Vec<String>,
    pub inventory: Inventory,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Inventory {
    pub updated_at: u64,
    pub containers: Vec<Container>,
    pub runtimes: Vec<String>,
    pub services: Vec<Service>,
    pub service_managers: Vec<String>,
    pub listeners: Vec<Listener>,
    pub notes: Vec<String>,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Snapshot {
    pub fn apply_inventory(&mut self, inventory: Inventory) {
        for service in &inventory.services {
            if service.active == "failed" {
                self.alerts.push(format!("Service {} failed", service.name));
            }
        }
        for container in &inventory.containers {
            if container.state == "restarting" || container.status.contains("unhealthy") {
                self.alerts.push(format!(
                    "Container {}: {}",
                    container.name, container.status
                ));
            }
        }
        let mut inventory = inventory;
        for service in &mut inventory.services {
            if let Some(process) = service
                .pid
                .and_then(|pid| self.processes.iter().find(|p| p.pid == pid))
            {
                service.cpu_percent = Some(process.cpu);
                if service.memory.is_none() {
                    service.memory = Some(process.memory);
                }
            }
        }
        self.inventory = inventory;
        crate::platform_gpu::merge(self);
        let mut groups = std::collections::BTreeMap::<String, (f32, u64, Vec<u32>)>::new();
        for process in &self.processes {
            if process.thread {
                continue;
            }
            let group = groups.entry(process.name.clone()).or_default();
            group.0 += process.cpu;
            group.1 = group.1.saturating_add(process.memory);
            group.2.push(process.pid);
        }
        self.extra.insert("Applications".into(),serde_json::json!(groups.into_iter().map(|(name,(cpu,rss,pids))| serde_json::json!({"name":name,"cpu_percent":cpu,"rss_sum_bytes":rss,"pids":pids})).collect::<Vec<_>>()));

        if let Some(text) = self
            .inventory
            .extra
            .get("Process TCP bytes / 1s (bpftrace)")
            .and_then(|v| v.get("output"))
            .and_then(|v| v.as_str())
        {
            for line in text.lines() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    for (direction, field) in [("@rx", true), ("@tx", false)] {
                        if let Some(values) = value["data"][direction].as_object() {
                            for (pid, bytes) in values {
                                if let Some(process) = pid.parse::<u32>().ok().and_then(|pid| {
                                    self.processes.iter_mut().find(|p| p.pid == pid)
                                }) {
                                    if field {
                                        process.rx_per_sec = bytes.as_f64();
                                    } else {
                                        process.tx_per_sec = bytes.as_f64();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Container {
    pub tags: Vec<crate::tags::Tag>,
    pub id: String,
    pub runtime: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub cpu: Option<f64>,
    pub memory: Option<u64>,
    pub memory_limit: Option<u64>,
    pub network_rx: Option<u64>,
    pub network_tx: Option<u64>,
    pub pids: Option<u64>,
    pub ports: String,
    pub command: String,
    pub created: String,
    pub running_for: String,
    pub memory_percent: Option<f64>,
    pub block_read: Option<u64>,
    pub block_write: Option<u64>,
    pub network_rx_per_sec: Option<f64>,
    pub network_tx_per_sec: Option<f64>,
    pub block_read_per_sec: Option<f64>,
    pub block_write_per_sec: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Service {
    pub tags: Vec<crate::tags::Tag>,
    pub name: String,
    pub scope: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
    pub pid: Option<u32>,
    pub cpu_percent: Option<f32>,
    pub memory: Option<u64>,
    pub cpu_time_ns: Option<u64>,
    pub tasks: Option<u64>,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Listener {
    pub protocol: String,
    pub address: String,
    pub port: u16,
    pub uid: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Memory {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub free: Option<u64>,
    pub cached: Option<u64>,
    pub swap_total: u64,
    pub swap_used: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Process {
    pub tags: Vec<crate::tags::Tag>,
    pub pid: u32,
    pub parent: Option<u32>,
    pub start_time: u64,
    pub name: String,
    pub command: String,
    pub user: String,
    pub state: String,
    /// One fully occupied logical core is 100% (htop convention).
    pub cpu: f32,
    pub memory: u64,
    pub virtual_memory: u64,
    pub cpu_time_ms: u64,
    pub read_per_sec: f64,
    pub write_per_sec: f64,
    pub read_total: u64,
    pub write_total: u64,
    pub gpu_memory: Option<u64>,
    pub gpu_percent: Option<u32>,
    pub gpu_ids: Vec<u32>,
    pub runtime: u64,
    pub thread: bool,
    pub kernel_thread: bool,
    pub nice: Option<i32>,
    pub rx_per_sec: Option<f64>,
    pub tx_per_sec: Option<f64>,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Network {
    pub name: String,
    pub mac: String,
    pub mtu: u64,
    pub speed_mbps: Option<u64>,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_packets_per_sec: Option<f64>,
    pub tx_packets_per_sec: Option<f64>,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_per_sec: f64,
    pub tx_per_sec: f64,
    pub rx_total: u64,
    pub tx_total: u64,
    pub errors: u64,
    pub drops: Option<u64>,
    pub state: String,
    pub address: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Disk {
    pub name: String,
    pub mount: String,
    pub filesystem: String,
    pub kind: String,
    pub removable: bool,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DiskIo {
    pub name: String,
    pub read_per_sec: f64,
    pub write_per_sec: f64,
    pub busy_percent: Option<f64>,
    pub aggregate: bool,
    pub read_iops: Option<f64>,
    pub write_iops: Option<f64>,
    pub await_ms: Option<f64>,
    pub queue_depth: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sensor {
    pub name: String,
    pub temperature: f32,
    pub critical: Option<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Battery {
    pub name: String,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
    pub percent: Option<f32>,
    pub status: String,
    pub watts: Option<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pressure {
    pub resource: String,
    pub some_10: Option<f64>,
    pub some_60: Option<f64>,
    pub some_300: Option<f64>,
    pub some_total_us: Option<u64>,
    pub full_10: Option<f64>,
    pub full_60: Option<f64>,
    pub full_300: Option<f64>,
    pub full_total_us: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Gpu {
    pub id: u32,
    pub name: String,
    pub vendor: String,
    pub backend: String,
    pub uuid: String,
    pub driver: String,
    pub cuda_driver_version: Option<String>,
    pub nvml_version: Option<String>,
    pub utilization: Option<u32>,
    pub memory_utilization: Option<u32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub temperature: Option<u32>,
    pub fan: Option<u32>,
    pub power_watts: Option<f32>,
    pub power_limit_watts: Option<f32>,
    pub clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub encoder: Option<u32>,
    pub decoder: Option<u32>,
    pub pcie_rx_kib: Option<u32>,
    pub pcie_tx_kib: Option<u32>,
    pub performance_state: Option<String>,
    pub processes: Vec<GpuProcess>,
    pub process_status: String,
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GpuProcess {
    pub pid: u32,
    pub memory: Option<u64>,
    pub utilization: Option<u32>,
    pub kind: String,
    pub memory_utilization: Option<u32>,
    pub encoder: Option<u32>,
    pub decoder: Option<u32>,
}

pub fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

pub fn bytes(value: f64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut number = value.max(0.0);
    let mut index = 0;
    while number >= 1024.0 && index < units.len() - 1 {
        number /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{number:.0} {}", units[index])
    } else {
        format!("{number:.1} {}", units[index])
    }
}

pub fn duration(seconds: u64) -> String {
    if seconds >= 86400 {
        format!("{}d {:02}h", seconds / 86400, seconds % 86400 / 3600)
    } else {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds % 3600 / 60,
            seconds % 60
        )
    }
}

pub fn is_loopback(name: &str) -> bool {
    name == "lo" || name == "lo0" || name.to_lowercase().contains("loopback")
}
