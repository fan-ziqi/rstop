use crate::model::*;
#[cfg(windows)]
use serde_json::Value;
#[cfg(target_os = "macos")]
use serde_json::json;
/// Apple counters are independent of optional system_profiler/powermetrics probes.
#[cfg(target_os = "macos")]
pub fn discover(
    config: &crate::config::Config,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut extra = std::collections::BTreeMap::new();
    if config.collect_gpu {
        let value = crate::discovery::command(
            "ioreg",
            &["-r", "-c", "IOAccelerator", "-l"],
            std::time::Duration::from_secs(3),
        )
        .map(|output| json!({"output":output,"sampled_at":crate::collector::unix_time()}))
        .unwrap_or_else(|error| json!({"error":error.to_string()}));
        extra.insert("Apple GPU / Neural Engine".into(), value);
    }
    extra
}

pub fn merge(snapshot: &mut Snapshot) {
    #[cfg(target_os = "macos")]
    {
        if let Some(value) = snapshot.inventory.extra.get("Apple GPU / Neural Engine")
            && value.get("error").is_none()
        {
            let text = value["output"].as_str().unwrap_or_default();
            if text.trim().is_empty() {
                return;
            }
            let metric = |name: &str| {
                regex::Regex::new(&format!(
                    "\\\"{}\\\"\\s*=\\s*([0-9.]+)",
                    regex::escape(name)
                ))
                .ok()?
                .captures(text)?
                .get(1)?
                .as_str()
                .parse::<f64>()
                .ok()
            };
            let cores = metric("gpu-core-count").map(|v| v as u32);
            let chip = snapshot.cpu_name.trim().to_string();
            let name = match (chip.is_empty(), cores) {
                (false, Some(n)) => format!("{chip} GPU ({n} cores)"),
                (false, None) => format!("{chip} GPU"),
                _ => "Apple GPU".to_string(),
            };
            let mut extra = std::collections::BTreeMap::new();
            extra.insert("IOKit performance statistics".into(), value.clone());
            extra.insert("memory_kind".into(), json!("unified system memory"));
            snapshot.gpus.push(Gpu {
                id: 10000,
                name,
                vendor: "Apple".into(),
                backend: "IOKit IOAccelerator".into(),
                uuid: "apple-iokit".into(),
                utilization: metric("Device Utilization %")
                    .or_else(|| metric("Renderer Utilization %"))
                    .map(|v| v as u32),
                memory_used: metric("In use system memory").map(|v| v as u64),
                memory_total: Some(snapshot.memory.total),
                extra,
                ..Default::default()
            });
            snapshot.gpu_status = "Apple GPU · IOKit IOAccelerator".into();
        }
    }
    #[cfg(windows)]
    {
        let adapters = snapshot
            .inventory
            .extra
            .get("Windows adapters")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let records = |name: &str| -> Vec<Value> {
            let Some(value) = snapshot.inventory.extra.get(name) else {
                return vec![];
            };
            value
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![value.clone()])
        };
        let engines = records("GPU engines and memory");
        let memories = records("GPU process memory");
        for (index, adapter) in adapters.iter().enumerate() {
            let vendor = match adapter["vendor_id"].as_u64() {
                Some(0x10de) => "NVIDIA",
                Some(0x1002) => "AMD",
                Some(0x8086) => "Intel",
                _ => "WDDM",
            };
            if vendor == "NVIDIA" && snapshot.gpus.iter().any(|g| g.vendor == "NVIDIA") {
                continue;
            }
            let luid = adapter["luid"].as_str().unwrap_or_default();
            let id = 10000 + index as u32;
            let mut per_engine = std::collections::HashMap::<String, u32>::new();
            let mut processes = std::collections::HashMap::<u32, GpuProcess>::new();
            for engine in &engines {
                let name = engine["Name"].as_str().unwrap_or_default();
                if !name.contains(luid) {
                    continue;
                }
                let util = engine["UtilizationPercentage"].as_u64().unwrap_or(0) as u32;
                let key = name.split("_eng_").nth(1).unwrap_or(name).to_string();
                *per_engine.entry(key).or_default() += util;
                if let Some(pid) = name
                    .strip_prefix("pid_")
                    .and_then(|s| s.split('_').next())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    let p = processes.entry(pid).or_insert_with(|| GpuProcess {
                        pid,
                        kind: "WDDM".into(),
                        ..Default::default()
                    });
                    p.utilization = Some(p.utilization.unwrap_or(0).max(util));
                }
            }
            for memory in &memories {
                let name = memory["Name"].as_str().unwrap_or_default();
                if !name.contains(luid) {
                    continue;
                }
                if let Some(pid) = name
                    .strip_prefix("pid_")
                    .and_then(|s| s.split('_').next())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    let p = processes.entry(pid).or_insert_with(|| GpuProcess {
                        pid,
                        kind: "WDDM".into(),
                        ..Default::default()
                    });
                    p.memory = memory["DedicatedUsage"].as_u64();
                }
            }
            let processes: Vec<_> = processes.into_values().collect();
            let used = processes.iter().filter_map(|p| p.memory).sum::<u64>();
            let dedicated = adapter["dedicated_bytes"].as_u64().unwrap_or(0);
            let mut extra = std::collections::BTreeMap::new();
            extra.insert("DXGI adapter".into(), adapter.clone());
            snapshot.gpus.push(Gpu {
                id,
                name: adapter["name"].as_str().unwrap_or("Windows GPU").into(),
                vendor: vendor.into(),
                backend: "DXGI + WDDM counters".into(),
                uuid: luid.into(),
                utilization: per_engine.values().copied().max().map(|v| v.min(100)),
                memory_used: if memories.is_empty() {
                    None
                } else {
                    Some(used)
                },
                memory_total: Some(if dedicated > 0 {
                    dedicated
                } else {
                    adapter["shared_bytes"].as_u64().unwrap_or(0)
                }),
                processes: processes.clone(),
                extra,
                ..Default::default()
            });
            for p in processes {
                if let Some(process) = snapshot
                    .processes
                    .iter_mut()
                    .find(|process| process.pid == p.pid)
                {
                    process.gpu_ids.push(id);
                    process.gpu_memory =
                        Some(process.gpu_memory.unwrap_or(0) + p.memory.unwrap_or(0));
                    process.gpu_percent =
                        Some(process.gpu_percent.unwrap_or(0) + p.utilization.unwrap_or(0));
                }
            }
        }
    }
    let _ = snapshot;
}
