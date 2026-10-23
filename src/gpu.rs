use crate::model::{Gpu, GpuProcess};
use nvml_wrapper::{
    Nvml,
    enum_wrappers::device::{Clock, PcieUtilCounter, TemperatureSensor},
    enums::device::UsedGpuMemory,
};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

#[derive(Default)]
struct NvmlDetails {
    at: Option<Instant>,
    values: BTreeMap<String, serde_json::Value>,
}
impl NvmlDetails {
    fn sample(&mut self, device: &nvml_wrapper::Device<'_>) -> BTreeMap<String, serde_json::Value> {
        if self
            .at
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(5))
        {
            self.values = advanced_nvml(device);
            self.at = Some(Instant::now());
        } else {
            // Discover metadata/capabilities less often; refresh every supported live counter.
            // Unsupported queries are retried on the next discovery, rather than every tick.
            use nvml_wrapper::enum_wrappers::device::{EccCounter, MemoryError};
            use serde_json::json;
            macro_rules! refresh {
                ($key:literal, $value:expr) => {
                    if self.values.get($key).is_some_and(|v| !v.is_null()) {
                        self.values.insert($key.into(), json!($value));
                    }
                };
            }
            refresh!(
                "bar1_memory",
                device.bar1_memory_info().ok().map(
                    |m| json!({"total_bytes":m.total,"used_bytes":m.used,"free_bytes":m.free})
                )
            );
            refresh!("pcie_current_gen", device.current_pcie_link_gen().ok());
            refresh!("pcie_current_width", device.current_pcie_link_width().ok());
            refresh!(
                "ecc_volatile_corrected",
                device
                    .total_ecc_errors(MemoryError::Corrected, EccCounter::Volatile)
                    .ok()
            );
            refresh!(
                "ecc_volatile_uncorrected",
                device
                    .total_ecc_errors(MemoryError::Uncorrected, EccCounter::Volatile)
                    .ok()
            );
            refresh!(
                "ecc_corrected",
                device
                    .total_ecc_errors(MemoryError::Corrected, EccCounter::Aggregate)
                    .ok()
            );
            refresh!(
                "ecc_uncorrected",
                device
                    .total_ecc_errors(MemoryError::Uncorrected, EccCounter::Aggregate)
                    .ok()
            );
            refresh!(
                "throttle_reasons",
                device
                    .current_throttle_reasons()
                    .ok()
                    .map(|reasons| format!("{reasons:?}")
                        .trim_start_matches("ThrottleReasons(")
                        .trim_end_matches(')')
                        .replace(" | ", ", "))
            );
            if let Some(links) = self.values.get_mut("nvlink").and_then(|v| v.as_array_mut()) {
                use nvml_wrapper::{enum_wrappers::nv_link::ErrorCounter, enums::nv_link::Counter};
                for value in links {
                    if let Some(index) = value["link"].as_u64() {
                        let link = device.link_wrapper_for(index as u32);
                        value["utilization_counters"] = json!(
                            link.utilization_counter(Counter::Zero)
                                .ok()
                                .map(|v| format!("{v:?}"))
                        );
                        value["replay_errors"] =
                            json!(link.error_counter(ErrorCounter::DlReplay).ok());
                    }
                }
            }
        }
        self.values.clone()
    }
}

#[derive(Default)]
pub struct Selection {
    pub min_free_bytes: u64,
    pub min_total_bytes: u64,
    pub max_gpu_utilization: Option<u32>,
    pub max_memory_utilization: Option<u32>,
}
pub fn select<'a>(gpus: &'a [Gpu], constraints: &Selection) -> Vec<&'a Gpu> {
    let mut eligible: Vec<_> = gpus
        .iter()
        .filter(|gpu| {
            gpu.vendor == "NVIDIA"
                && !gpu.uuid.is_empty()
                && gpu
                    .extra
                    .get("mig_children")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    == 0
                && gpu
                    .memory_total
                    .zip(gpu.memory_used)
                    .is_some_and(|(total, used)| {
                        total >= constraints.min_total_bytes
                            && total.saturating_sub(used) >= constraints.min_free_bytes
                    })
                && constraints
                    .max_gpu_utilization
                    .is_none_or(|limit| gpu.utilization.is_some_and(|value| value <= limit))
                && constraints
                    .max_memory_utilization
                    .is_none_or(|limit| gpu.memory_utilization.is_some_and(|value| value <= limit))
        })
        .collect();
    eligible.sort_by_key(|gpu| {
        (
            gpu.utilization.unwrap_or(101),
            gpu.memory_utilization.unwrap_or(101),
            std::cmp::Reverse(
                gpu.memory_total
                    .unwrap_or(0)
                    .saturating_sub(gpu.memory_used.unwrap_or(0)),
            ),
            gpu.id,
        )
    });
    eligible
}

struct ProcessSample {
    uuid: String,
    processes: Vec<GpuProcess>,
    errors: Vec<String>,
    timestamp: u64,
}
pub struct GpuCollector {
    nvml: Option<Nvml>,
    status: String,
    last_seen: HashMap<u32, u64>,
    enabled: bool,
    details: HashMap<u32, NvmlDetails>,
    processes: HashMap<u32, ProcessSample>,
}

impl GpuCollector {
    pub fn new(enabled: bool) -> Self {
        if !enabled {
            return Self {
                nvml: None,
                status: "GPU collection disabled (--no-gpu)".into(),
                last_seen: HashMap::new(),
                enabled,
                details: HashMap::new(),
                processes: HashMap::new(),
            };
        }
        if cfg!(target_os = "macos") {
            return Self {
                nvml: None,
                status: "NVML not supported on macOS".into(),
                last_seen: HashMap::new(),
                enabled,
                details: HashMap::new(),
                processes: HashMap::new(),
            };
        }
        match Nvml::init() {
            Ok(nvml) => Self {
                nvml: Some(nvml),
                status: "NVML ready".into(),
                last_seen: HashMap::new(),
                enabled,
                details: HashMap::new(),
                processes: HashMap::new(),
            },
            Err(error) => Self {
                nvml: None,
                status: format!("NVML unavailable: {error}"),
                last_seen: HashMap::new(),
                enabled,
                details: HashMap::new(),
                processes: HashMap::new(),
            },
        }
    }

    pub fn collect(&mut self) -> (Vec<Gpu>, String) {
        self.collect_with_processes(true)
    }

    pub fn collect_with_processes(&mut self, live_processes: bool) -> (Vec<Gpu>, String) {
        if !self.enabled {
            return (vec![], self.status.clone());
        }
        let mut gpus = vec![];
        let mut issues = vec![];
        if let Some(nvml) = &self.nvml {
            let driver = nvml.sys_driver_version().unwrap_or_default();
            let cuda_driver_version = nvml.sys_cuda_driver_version().ok().map(|v| {
                format!(
                    "{}.{}",
                    nvml_wrapper::cuda_driver_version_major(v),
                    nvml_wrapper::cuda_driver_version_minor(v)
                )
            });
            let nvml_version = nvml.sys_nvml_version().ok();
            match nvml.device_count() {
                Err(error) => issues.push(format!("NVML device discovery: {error}")),
                Ok(count) => {
                    for id in 0..count {
                        let device = match nvml.device_by_index(id) {
                            Ok(device) => device,
                            Err(error) => {
                                issues.push(format!("GPU {id}: {error}"));
                                continue;
                            }
                        };
                        let utilization = device.utilization_rates().ok();
                        let memory = device.memory_info().ok();
                        let uuid = device.uuid().unwrap_or_default();
                        if self.processes.get(&id).is_some_and(|old| old.uuid != uuid) {
                            self.processes.remove(&id);
                            self.details.remove(&id);
                            self.last_seen.remove(&id);
                        }
                        let refresh = live_processes;
                        let (processes, process_errors) = if refresh {
                            let samples = device
                                .process_utilization_stats(self.last_seen.get(&id).copied())
                                .ok()
                                .map(|items| {
                                    items
                                        .into_iter()
                                        .filter(|sample| {
                                            sample.sm_util <= 100
                                                && sample.mem_util <= 100
                                                && sample.enc_util <= 100
                                                && sample.dec_util <= 100
                                                && sample.timestamp
                                                    > self.last_seen.get(&id).copied().unwrap_or(0)
                                        })
                                        .collect::<Vec<_>>()
                                });
                            if let Some(latest) = samples
                                .as_ref()
                                .and_then(|items| items.iter().map(|s| s.timestamp).max())
                            {
                                self.last_seen.insert(id, latest);
                            }
                            let mut by_pid: HashMap<u32, GpuProcess> = HashMap::new();
                            let mut process_errors = vec![];
                            // v3 -> v2 fallback supports older drivers. Compute + graphics entries
                            // for the same PID describe the same allocation, so never add twice.
                            for (kind, result) in [
                                (
                                    "C",
                                    device
                                        .running_compute_processes()
                                        .or_else(|_| device.running_compute_processes_v2()),
                                ),
                                (
                                    "G",
                                    device
                                        .running_graphics_processes()
                                        .or_else(|_| device.running_graphics_processes_v2()),
                                ),
                            ] {
                                match result {
                                    Ok(processes) => {
                                        for process in processes {
                                            let used = match process.used_gpu_memory {
                                                UsedGpuMemory::Used(n) => Some(n),
                                                _ => None,
                                            };
                                            let entry =
                                                by_pid.entry(process.pid).or_insert_with(|| {
                                                    GpuProcess {
                                                        pid: process.pid,
                                                        memory: used,
                                                        kind: String::new(),
                                                        utilization: None,
                                                        ..GpuProcess::default()
                                                    }
                                                });
                                            entry.memory = entry.memory.max(used);
                                            entry.kind.push_str(kind);
                                        }
                                    }
                                    Err(error) => process_errors.push(format!("{kind}: {error}")),
                                }
                            }
                            if let Some(samples) = samples {
                                let mut latest = HashMap::new();
                                for sample in samples {
                                    let entry = latest.entry(sample.pid).or_insert((0, 0));
                                    if sample.timestamp >= entry.0 {
                                        *entry = (sample.timestamp, sample.sm_util);
                                        if let Some(p) = by_pid.get_mut(&sample.pid) {
                                            p.memory_utilization = Some(sample.mem_util);
                                            p.encoder = Some(sample.enc_util);
                                            p.decoder = Some(sample.dec_util);
                                        }
                                    }
                                }
                                for (pid, (_, utilization)) in latest {
                                    if let Some(process) = by_pid.get_mut(&pid) {
                                        process.utilization = Some(utilization);
                                    }
                                }
                            }
                            let mut processes: Vec<_> = by_pid.into_values().collect();
                            processes.sort_by_key(|p| std::cmp::Reverse(p.memory.unwrap_or(0)));
                            self.processes.insert(
                                id,
                                ProcessSample {
                                    uuid: uuid.clone(),
                                    processes: processes.clone(),
                                    errors: process_errors.clone(),
                                    timestamp: crate::collector::unix_time(),
                                },
                            );
                            (processes, process_errors)
                        } else {
                            self.processes
                                .get(&id)
                                .filter(|cached| cached.uuid == uuid)
                                .map(|cached| (cached.processes.clone(), cached.errors.clone()))
                                .unwrap_or_default()
                        };
                        let mut extra = if live_processes {
                            self.details.entry(id).or_default().sample(&device)
                        } else {
                            BTreeMap::new()
                        };
                        if let Some(cached) = self.processes.get(&id) {
                            extra.insert(
                                "process_sampled_at".into(),
                                serde_json::json!(cached.timestamp),
                            );
                        }
                        gpus.push(Gpu {
                            id,
                            name: device.name().unwrap_or_else(|_| format!("NVIDIA GPU {id}")),
                            vendor: "NVIDIA".into(),
                            backend: "NVML".into(),
                            uuid,
                            driver: driver.clone(),
                            cuda_driver_version:cuda_driver_version.clone(),nvml_version:nvml_version.clone(),
                            utilization: utilization.as_ref().map(|u| u.gpu),
                            memory_utilization: utilization.map(|u| u.memory),
                            memory_used: memory.as_ref().map(|m| m.used),
                            memory_total: memory.map(|m| m.total),
                            temperature: device.temperature(TemperatureSensor::Gpu).ok(),
                            fan: device.fan_speed(0).ok(),
                            power_watts: device.power_usage().ok().map(|v| v as f32 / 1000.0),
                            power_limit_watts: device
                                .enforced_power_limit()
                                .ok()
                                .map(|v| v as f32 / 1000.0),
                            clock_mhz: device.clock_info(Clock::Graphics).ok(),
                            memory_clock_mhz: device.clock_info(Clock::Memory).ok(),
                            encoder: device.encoder_utilization().ok().map(|u| u.utilization),
                            decoder: device.decoder_utilization().ok().map(|u| u.utilization),
                            pcie_rx_kib: device.pcie_throughput(PcieUtilCounter::Receive).ok(),
                            pcie_tx_kib: device.pcie_throughput(PcieUtilCounter::Send).ok(),
                            performance_state: device
                                .performance_state()
                                .ok()
                                .and_then(|s|if s==nvml_wrapper::enum_wrappers::device::PerformanceState::Unknown {None}else{Some(format!("P{}",s as u32))}),
                            processes,
                            extra,
                            process_status: if !live_processes && !self.processes.contains_key(&id) {
                                "Not sampled · open Processes or GPU".into()
                            } else if process_errors.is_empty() {
                                "compute + graphics".into()
                            } else {
                                process_errors.join("; ")
                            },
                            ..Default::default()
                        });
                    }
                }
            }
        }
        if live_processes && let Some(nvml) = &self.nvml {
            let mut migs = vec![];
            for parent in &mut gpus {
                let Ok(device) = nvml.device_by_index(parent.id) else {
                    continue;
                };
                let Ok(count) = device.mig_device_count() else {
                    continue;
                };
                for index in 0..count {
                    let Ok(mig) = device.mig_device_by_index(index) else {
                        continue;
                    };
                    let memory = mig.memory_info().ok();
                    let utilization = mig.utilization_rates().ok();
                    let processes = mig
                        .running_compute_processes()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| GpuProcess {
                            pid: p.pid,
                            memory: match p.used_gpu_memory {
                                UsedGpuMemory::Used(v) => Some(v),
                                _ => None,
                            },
                            kind: "MIG".into(),
                            ..Default::default()
                        })
                        .collect();
                    let mut extra = self
                        .details
                        .entry(1_000_000 + parent.id * 1000 + index)
                        .or_default()
                        .sample(&mig);
                    extra.insert("parent_gpu".into(), serde_json::json!(parent.id));
                    migs.push(Gpu {
                        id: 1_000_000 + parent.id * 1000 + index,
                        name: mig
                            .name()
                            .unwrap_or_else(|_| format!("MIG {}:{index}", parent.id)),
                        uuid: mig.uuid().unwrap_or_default(),
                        vendor: "NVIDIA".into(),
                        backend: "NVML MIG".into(),
                        driver: parent.driver.clone(),
                        cuda_driver_version: parent.cuda_driver_version.clone(),
                        nvml_version: parent.nvml_version.clone(),
                        memory_used: memory.as_ref().map(|m| m.used),
                        memory_total: memory.map(|m| m.total),
                        utilization: utilization.map(|u| u.gpu),
                        processes,
                        process_status: "MIG compute processes".into(),
                        extra,
                        ..Default::default()
                    });
                }
                let children = migs
                    .iter()
                    .filter(|g| {
                        g.extra.get("parent_gpu").and_then(|v| v.as_u64()) == Some(parent.id as u64)
                    })
                    .count();
                parent
                    .extra
                    .insert("mig_children".into(), serde_json::json!(children));
            }
            gpus.extend(migs);
        }
        #[cfg(target_os = "linux")]
        gpus.extend(sysfs_gpus(gpus.len() as u32));
        self.details
            .retain(|id, _| gpus.iter().any(|g| g.id == *id));
        self.processes
            .retain(|id, _| gpus.iter().any(|g| g.id == *id));
        let status = if !issues.is_empty() {
            issues.join("; ")
        } else if gpus.is_empty() {
            format!("No readable GPU telemetry. {}", self.status)
        } else {
            format!("{} device(s) · unsupported metrics = —", gpus.len())
        };
        (gpus, status)
    }
}

#[cfg(target_os = "linux")]
fn sysfs_gpus(first_id: u32) -> Vec<Gpu> {
    use std::{fs, path::Path};
    fn read(path: impl AsRef<Path>) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }
    fn number(path: impl AsRef<Path>) -> Option<u64> {
        read(path)?.parse().ok()
    }
    let mut cards: Vec<_> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix("card")
                .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    cards.sort_by_key(|e| e.file_name());
    let mut gpus = vec![];
    for card in cards {
        let device = card.path().join("device");
        let vendor = read(device.join("vendor")).unwrap_or_default();
        if vendor == "0x10de" && first_id > 0 {
            continue;
        }
        let vendor_name = match vendor.as_str() {
            "0x1002" => "AMD",
            "0x8086" => "Intel",
            "0x10de" => "NVIDIA",
            "0x5143" => "Qualcomm",
            "0x14e4" => "Broadcom",
            "0x13b5" => "Arm",
            _ => "DRM",
        };
        let hwmon = fs::read_dir(device.join("hwmon"))
            .ok()
            .and_then(|mut d| d.next())
            .and_then(Result::ok)
            .map(|e| e.path());
        let temp = hwmon
            .as_ref()
            .and_then(|p| number(p.join("temp1_input")))
            .map(|n| (n / 1000) as u32);
        let power = hwmon
            .as_ref()
            .and_then(|p| number(p.join("power1_average")))
            .map(|n| n as f32 / 1e6);
        let model = read(device.join("product_name")).unwrap_or_else(|| {
            format!(
                "{} {}",
                vendor_name,
                read(device.join("device")).unwrap_or_default()
            )
        });
        gpus.push(Gpu {
            id: first_id + gpus.len() as u32,
            name: model,
            vendor: vendor_name.into(),
            backend: "DRM sysfs".into(),
            driver: fs::read_link(device.join("driver"))
                .ok()
                .and_then(|p| {
                    p.file_name().map(|s| {
                        format!(
                            "{} / kernel {}",
                            s.to_string_lossy(),
                            sysinfo::System::kernel_version().unwrap_or_default()
                        )
                    })
                })
                .unwrap_or_default(),
            extra: std::collections::BTreeMap::from([
                (
                    "pci_bus_id".into(),
                    serde_json::json!(
                        fs::canonicalize(&device)
                            .ok()
                            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                    ),
                ),
                (
                    "shared_memory_used_bytes".into(),
                    serde_json::json!(number(device.join("mem_info_gtt_used"))),
                ),
                (
                    "shared_memory_total_bytes".into(),
                    serde_json::json!(number(device.join("mem_info_gtt_total"))),
                ),
                (
                    "pcie_link_speed".into(),
                    serde_json::json!(read(device.join("current_link_speed"))),
                ),
                (
                    "pcie_link_width".into(),
                    serde_json::json!(read(device.join("current_link_width"))),
                ),
            ]),
            uuid: fs::canonicalize(&device)
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_default(),
            utilization: number(device.join("gpu_busy_percent")).map(|n| n as u32),
            memory_used: number(device.join("mem_info_vram_used")),
            memory_total: number(device.join("mem_info_vram_total")),
            temperature: temp,
            power_watts: power,
            clock_mhz: number(card.path().join("gt_cur_freq_mhz")).map(|n| n as u32),
            process_status: "DRM fdinfo readable clients; engine counters require two samples"
                .into(),
            ..Gpu::default()
        });
    }
    gpus
}

fn advanced_nvml(
    device: &nvml_wrapper::Device<'_>,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    use nvml_wrapper::enum_wrappers::device::{EccCounter, MemoryError};
    use nvml_wrapper::{enum_wrappers::nv_link::ErrorCounter, enums::nv_link::Counter};
    use serde_json::json;
    let mut extra = std::collections::BTreeMap::new();
    extra.insert(
        "pci_bus_id".into(),
        json!(device.pci_info().ok().map(|p| p.bus_id)),
    );
    #[cfg(target_os = "linux")]
    extra.insert(
        "persistence_mode".into(),
        json!(device.is_in_persistent_mode().ok()),
    );
    #[cfg(not(target_os = "linux"))]
    extra.insert("persistence_mode".into(), serde_json::Value::Null);
    extra.insert(
        "display_active".into(),
        json!(device.is_display_active().ok()),
    );
    extra.insert(
        "display_connected".into(),
        json!(device.is_display_connected().ok()),
    );
    extra.insert(
        "compute_mode".into(),
        json!(device.compute_mode().ok().map(|v| format!("{v:?}"))),
    );
    extra.insert("vbios_version".into(), json!(device.vbios_version().ok()));
    extra.insert(
        "architecture".into(),
        json!(device.architecture().ok().map(|v| format!("{v:?}"))),
    );
    extra.insert(
        "brand".into(),
        json!(device.brand().ok().map(|v| format!("{v:?}"))),
    );
    extra.insert(
        "bar1_memory".into(),
        json!(
            device
                .bar1_memory_info()
                .ok()
                .map(|m| json!({"total_bytes":m.total,"used_bytes":m.used,"free_bytes":m.free}))
        ),
    );
    extra.insert(
        "memory_bus_width_bits".into(),
        json!(device.memory_bus_width().ok()),
    );
    extra.insert(
        "pcie_current_gen".into(),
        json!(device.current_pcie_link_gen().ok()),
    );
    extra.insert(
        "pcie_max_gen".into(),
        json!(device.max_pcie_link_gen().ok()),
    );
    extra.insert(
        "pcie_current_width".into(),
        json!(device.current_pcie_link_width().ok()),
    );
    extra.insert(
        "pcie_max_width".into(),
        json!(device.max_pcie_link_width().ok()),
    );
    extra.insert(
        "max_graphics_clock_mhz".into(),
        json!(device.max_clock_info(Clock::Graphics).ok()),
    );
    extra.insert(
        "max_memory_clock_mhz".into(),
        json!(device.max_clock_info(Clock::Memory).ok()),
    );
    extra.insert(
        "ecc_volatile_corrected".into(),
        json!(
            device
                .total_ecc_errors(MemoryError::Corrected, EccCounter::Volatile)
                .ok()
        ),
    );
    extra.insert(
        "ecc_volatile_uncorrected".into(),
        json!(
            device
                .total_ecc_errors(MemoryError::Uncorrected, EccCounter::Volatile)
                .ok()
        ),
    );
    if let Ok(reasons) = device.current_throttle_reasons() {
        extra.insert(
            "throttle_reasons".into(),
            json!(
                format!("{reasons:?}")
                    .trim_start_matches("ThrottleReasons(")
                    .trim_end_matches(')')
                    .replace(" | ", ", ")
            ),
        );
    }
    if let Ok(ecc) = device.is_ecc_enabled() {
        extra.insert(
            "ecc_mode".into(),
            json!({"current":ecc.currently_enabled,"pending":ecc.pending_enabled}),
        );
    }
    for (name, kind) in [
        ("ecc_corrected", MemoryError::Corrected),
        ("ecc_uncorrected", MemoryError::Uncorrected),
    ] {
        if let Ok(count) = device.total_ecc_errors(kind, EccCounter::Aggregate) {
            extra.insert(name.into(), json!(count));
        }
    }
    if let Ok(mode) = device.mig_mode() {
        extra.insert(
            "mig_mode".into(),
            json!({"current":mode.current,"pending":mode.pending}),
        );
    }
    let mut links = vec![];
    for index in 0..18 {
        let link = device.link_wrapper_for(index);
        match link.is_active() {
            Ok(true) => {}
            Err(
                nvml_wrapper::error::NvmlError::NotSupported
                | nvml_wrapper::error::NvmlError::InvalidArg,
            ) => break,
            _ => continue,
        }
        links.push(json!({"link":index,"version":link.version().ok(),"remote":link.remote_pci_info().ok().map(|p|p.bus_id),"utilization_counters":link.utilization_counter(Counter::Zero).ok().map(|v|format!("{v:?}")),"counter_control":link.utilization_control(Counter::Zero).ok().map(|v|format!("{v:?}")),"replay_errors":link.error_counter(ErrorCounter::DlReplay).ok()}));
    }
    if !links.is_empty() {
        extra.insert("nvlink".into(), json!(links));
    }
    extra
}
