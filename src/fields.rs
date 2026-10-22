//! Human-readable field browsers; raw backend keys remain searchable.
use crate::model::{Gpu, bytes};
use serde_json::Value;
pub type Field = (String, String, String);
pub fn value(v: &Value, key: &str) -> String {
    match v {
        Value::Null => "— unavailable".into(),
        Value::Bool(v) => if *v { "On" } else { "Off" }.into(),
        Value::String(v) => v.clone(),
        Value::Number(n) => {
            if (key.ends_with("_bytes")
                || ["memory_used", "memory_total", "memory_free"].contains(&key))
                && let Some(n) = n.as_f64()
            {
                return bytes(n);
            }
            if let Some(n) = n.as_f64() {
                if key.ends_with("_mhz") {
                    return format!("{n:.0} MHz");
                }
                if key.ends_with("_watts") {
                    return format!("{n:.1} W");
                }
            }
            if n.is_f64() {
                format!("{:.2}", n.as_f64().unwrap_or(0.))
            } else {
                n.to_string()
            }
        }
        _ => v.to_string(),
    }
}
pub fn label(key: &str) -> String {
    match key {
        "cuda_driver_version" => "CUDA Driver API version (maximum supported)".into(),
        "driver" => "GPU driver version".into(),
        "pci_bus_id" => "PCI Bus-ID".into(),
        "nvml_version" => "NVML version".into(),
        "ecc_volatile_uncorrected" => "Volatile uncorrectable ECC errors".into(),
        "uuid" => "UUID".into(),
        other => other.replace('_', " "),
    }
}
fn walk(v: &Value, key: &str, group: &str, out: &mut Vec<Field>) {
    match v {
        Value::Object(values) => {
            for (name, value) in values {
                walk(
                    value,
                    &if key.is_empty() {
                        name.clone()
                    } else {
                        format!("{key} / {name}")
                    },
                    group,
                    out,
                );
            }
        }
        Value::Array(values) => {
            if values.is_empty() {
                out.push((group.into(), label(key), "None".into()));
            } else {
                for (i, value) in values.iter().enumerate() {
                    walk(value, &format!("{key} #{}", i + 1), group, out);
                }
            }
        }
        _ => out.push((group.into(), label(key), value(v, key))),
    }
}
pub fn gpu(gpu: &Gpu, filter: &str) -> Vec<Field> {
    let mut rows = vec![];
    let data = serde_json::to_value(gpu).unwrap_or_default();
    if let Some(values) = data.as_object() {
        for (key, v) in values {
            if key == "processes" || key == "extra" {
                continue;
            }
            let group = match key.as_str() {
                "id"
                | "name"
                | "vendor"
                | "backend"
                | "uuid"
                | "driver"
                | "cuda_driver_version"
                | "nvml_version" => "1 · Identity & software",
                "memory_used" | "memory_total" | "memory_utilization" => "3 · Memory",
                "clock_mhz" | "memory_clock_mhz" => "5 · Clocks",
                "temperature" | "fan" | "power_watts" | "power_limit_watts" => {
                    "6 · Power & cooling"
                }
                "pcie_rx_kib" | "pcie_tx_kib" => "7 · PCIe",
                _ => "4 · Engines & processes",
            };
            rows.push((
                group.into(),
                label(key),
                match key.as_str() {
                    "utilization" | "memory_utilization" | "fan" | "encoder" | "decoder" => v
                        .as_u64()
                        .map(|v| format!("{v}%"))
                        .unwrap_or_else(|| value(v, key)),
                    "temperature" => v
                        .as_u64()
                        .map(|v| format!("{v} °C"))
                        .unwrap_or_else(|| value(v, key)),
                    "pcie_rx_kib" | "pcie_tx_kib" => v
                        .as_u64()
                        .map(|v| format!("{}/s", bytes(v as f64 * 1024.)))
                        .unwrap_or_else(|| value(v, key)),
                    _ => value(v, key),
                },
            ));
        }
    }
    for (key, v) in &gpu.extra {
        let group = if key.starts_with("nvlink") {
            "8 · NVLink"
        } else if key.starts_with("pcie") || key.starts_with("pci_") {
            "7 · PCIe"
        } else if key.contains("ecc")
            || key.contains("mode")
            || key.contains("display")
            || key.contains("throttle")
        {
            "2 · Modes & health"
        } else if key.contains("clock") {
            "5 · Clocks"
        } else if key.contains("memory") {
            "3 · Memory"
        } else {
            "1 · Identity & software"
        };
        walk(v, key, group, &mut rows);
    }
    if let Some((used, total)) = gpu.memory_used.zip(gpu.memory_total) {
        rows.push((
            "3 · Memory".into(),
            "memory free".into(),
            bytes(total.saturating_sub(used) as f64),
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let filter = filter.to_lowercase();
    rows.retain(|(group, name, value)| {
        format!("{group} {name} {value}")
            .to_lowercase()
            .contains(&filter)
    });
    rows
}
pub fn pretty(v: &Value) -> String {
    let mut rows = vec![];
    walk(v, "", "", &mut rows);
    rows.into_iter()
        .map(|(_, name, value)| format!("{name:<32} {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub const PROCESS_COLUMNS: &[&str] = &[
    "pid",
    "ppid",
    "user",
    "uid",
    "cpu",
    "cpu_normalized",
    "memory",
    "memory_percent",
    "vms",
    "vram",
    "gpu",
    "gpu_mem",
    "encoder",
    "decoder",
    "type",
    "read",
    "write",
    "read_total",
    "write_total",
    "rx",
    "tx",
    "state",
    "nice",
    "priority",
    "processor",
    "threads",
    "tgid",
    "pgrp",
    "session",
    "tty",
    "time",
    "runtime",
    "start",
    "utime",
    "stime",
    "cutime",
    "cstime",
    "minor_faults",
    "major_faults",
    "shared",
    "private",
    "data",
    "code",
    "lib",
    "swap",
    "pss",
    "pss_swap",
    "epss",
    "uss",
    "oom",
    "io_priority",
    "context_switches",
    "rchar",
    "wchar",
    "syscr",
    "syscw",
    "cancelled_write",
    "cgroup",
    "autogroup",
    "autogroup_nice",
    "cpu_delay",
    "io_delay",
    "swap_delay",
    "command",
    "tags",
    "comm",
    "exe",
    "cwd",
    "security",
    "wchan",
    "tpgid",
    "minor_faults_children",
    "major_faults_children",
    "rt_priority",
    "scheduling",
    "uid_effective",
    "uid_saved",
    "gid",
    "gid_effective",
    "nspid",
    "seccomp",
    "capabilities",
    "voluntary_context_switches",
    "nonvoluntary_context_switches",
    "io_total_rate",
    "cpu_time_children",
    "gpu_time",
    "exe_deleted",
    "fd_count",
    "autogroup_id",
];

pub fn process_metric(
    p: &crate::model::Process,
    s: &crate::model::Snapshot,
    key: &str,
) -> Option<Value> {
    let stat = p.extra.get("stat").unwrap_or(&Value::Null);
    let status = p.extra.get("status").unwrap_or(&Value::Null);
    let io = p.extra.get("io").unwrap_or(&Value::Null);
    let smaps = p.extra.get("smaps_rollup").unwrap_or(&Value::Null);
    let num = |v: f64| serde_json::Number::from_f64(v).map(Value::Number);
    let integer = |v: u64| Some(Value::from(v));
    match key {
        "tags" => Some(Value::String(crate::tags::text(&p.tags))),
        "comm" | "exe" | "cwd" | "security" | "wchan" | "exe_deleted" | "fd_count" => {
            p.extra.get(key).filter(|v| !v.is_null()).cloned()
        }
        "gpu_time" => p
            .extra
            .get("drm_engine_time_ns")
            .and_then(Value::as_f64)
            .and_then(|v| num(v / 1e9)),
        "cpu_time_children" => stat
            .get("children_utime_ticks")
            .and_then(Value::as_f64)
            .zip(stat.get("children_stime_ticks").and_then(Value::as_f64))
            .and_then(|(a, b)| {
                num((a + b)
                    / p.extra
                        .get("clock_ticks_per_second")
                        .and_then(Value::as_f64)
                        .unwrap_or(100.))
            }),
        "io_total_rate" => num(p.read_per_sec + p.write_per_sec),
        "uid_effective" | "uid_saved" | "gid" | "gid_effective" => status
            .get(if key.starts_with("uid") { "Uid" } else { "Gid" })
            .and_then(Value::as_str)
            .and_then(|v| {
                v.split_whitespace().nth(match key {
                    "uid_effective" | "gid_effective" => 1,
                    "uid_saved" => 2,
                    _ => 0,
                })
            })
            .and_then(|v| v.parse::<u64>().ok())
            .map(Value::from),
        "nspid"
        | "seccomp"
        | "capabilities"
        | "voluntary_context_switches"
        | "nonvoluntary_context_switches" => status
            .get(match key {
                "nspid" => "NSpid",
                "seccomp" => "Seccomp",
                "capabilities" => "CapEff",
                "voluntary_context_switches" => "voluntary_ctxt_switches",
                _ => "nonvoluntary_ctxt_switches",
            })
            .filter(|v| !v.is_null())
            .cloned(),
        "autogroup_id" => p
            .extra
            .get("autogroup")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("/autogroup-"))
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Value::from),
        "tpgid"
        | "minor_faults_children"
        | "major_faults_children"
        | "rt_priority"
        | "scheduling" => stat.get(key).filter(|v| !v.is_null()).cloned(),
        "ppid" => p.parent.map(Value::from),
        "cpu_normalized" => num(p.cpu as f64 / s.cores.len().max(1) as f64),
        "memory_percent" => num(crate::model::percent(p.memory, s.memory.total)),
        "runtime" => integer(p.runtime),
        "start" => integer(p.start_time),
        "priority" | "processor" | "threads" | "pgrp" | "session" | "tty" | "minor_faults"
        | "major_faults" => stat.get(key).filter(|v| !v.is_null()).cloned(),
        "utime" | "stime" | "cutime" | "cstime" => stat
            .get(match key {
                "cutime" => "children_utime_ticks".into(),
                "cstime" => "children_stime_ticks".into(),
                _ => format!("{key}_ticks"),
            })
            .and_then(Value::as_f64)
            .and_then(|v| {
                num(v / p
                    .extra
                    .get("clock_ticks_per_second")
                    .and_then(Value::as_f64)
                    .unwrap_or(100.))
            }),
        "uid" => status
            .get("Uid")
            .and_then(Value::as_str)
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Value::from),
        "tgid" => status.get("Tgid").cloned(),
        "data" => status.get("VmData").cloned(),
        "code" => status.get("VmExe").cloned(),
        "lib" => status.get("VmLib").cloned(),
        "swap" => status.get("VmSwap").cloned(),
        "shared" => p.extra.get("shared_bytes").cloned(),
        "private" => p
            .extra
            .get("shared_bytes")
            .and_then(Value::as_u64)
            .and_then(|shared| integer(p.memory.saturating_sub(shared))),
        "pss" => smaps.get("Pss").cloned(),
        "pss_swap" => smaps.get("SwapPss").cloned(),
        "epss" => smaps
            .get("Pss")
            .and_then(Value::as_u64)
            .zip(smaps.get("SwapPss").and_then(Value::as_u64))
            .and_then(|(a, b)| integer(a.saturating_add(b))),
        "uss" => smaps
            .get("Private_Clean")
            .and_then(Value::as_u64)
            .zip(smaps.get("Private_Dirty").and_then(Value::as_u64))
            .and_then(|(a, b)| integer(a.saturating_add(b))),
        "context_switches" => status
            .get("voluntary_ctxt_switches")
            .and_then(Value::as_u64)
            .zip(
                status
                    .get("nonvoluntary_ctxt_switches")
                    .and_then(Value::as_u64),
            )
            .and_then(|(a, b)| integer(a + b)),
        "rchar" | "wchar" | "syscr" | "syscw" => io.get(key).cloned(),
        "cancelled_write" => io.get("cancelled_write_bytes").cloned(),
        "oom" | "io_priority" | "cgroup" | "autogroup" | "autogroup_nice" | "cpu_delay"
        | "io_delay" | "swap_delay" => p.extra.get(key).filter(|v| !v.is_null()).cloned(),
        _ => None,
    }
}
pub fn process_display(p: &crate::model::Process, s: &crate::model::Snapshot, key: &str) -> String {
    let Some(v) = process_metric(p, s, key) else {
        return "—".into();
    };
    if [
        "shared",
        "private",
        "data",
        "code",
        "lib",
        "swap",
        "pss",
        "pss_swap",
        "epss",
        "uss",
        "rchar",
        "wchar",
        "cancelled_write",
    ]
    .contains(&key)
    {
        return v.as_f64().map(bytes).unwrap_or_else(|| value(&v, key));
    }
    if [
        "utime",
        "stime",
        "cutime",
        "cstime",
        "runtime",
        "cpu_time_children",
        "gpu_time",
    ]
    .contains(&key)
    {
        return v
            .as_f64()
            .map(|n| format!("{n:.1}s"))
            .unwrap_or_else(|| value(&v, key));
    }
    if [
        "cpu_normalized",
        "memory_percent",
        "cpu_delay",
        "io_delay",
        "swap_delay",
    ]
    .contains(&key)
    {
        return v
            .as_f64()
            .map(|n| format!("{n:.1}%"))
            .unwrap_or_else(|| value(&v, key));
    }
    if key == "io_total_rate" {
        return v
            .as_f64()
            .map(|v| format!("{}/s", bytes(v)))
            .unwrap_or_else(|| "—".into());
    }
    if key == "scheduling" {
        return v
            .as_u64()
            .map(|v| {
                match v {
                    0 => "Normal",
                    1 => "FIFO",
                    2 => "Round robin",
                    3 => "Batch",
                    5 => "Idle",
                    6 => "Deadline",
                    _ => "Unknown",
                }
                .into()
            })
            .unwrap_or_else(|| "—".into());
    }
    value(&v, key)
}

pub fn column_label(key: &str) -> String {
    match key {
        "memory_percent" => "RAM%".into(),
        "cpu_normalized" => "CPU/T%".into(),
        "gpu_mem" => "GMBW%".into(),
        "vram" => "GPU-MEM".into(),
        "runtime" => "ELAPSED".into(),
        "time" => "CPU TIME".into(),
        "context_switches" => "CTXT TOTAL".into(),
        "encoder" => "ENC%".into(),
        "decoder" => "DEC%".into(),
        other => other.to_uppercase(),
    }
}
