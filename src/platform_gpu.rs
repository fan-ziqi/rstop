use crate::model::*;
#[cfg(windows)]
use serde_json::Value;
#[cfg(target_os = "macos")]
use serde_json::json;
/// Reads the accelerator's `PerformanceStatistics` and `gpu-core-count` straight from the IOKit
/// registry. Spawning `ioreg` serializes the whole registry entry (about 50 KB, ~19 ms of CPU) every
/// sample; asking IOKit for two properties costs microseconds. The text mimics `ioreg` output so the
/// parser in `merge` and the saved recordings keep working.
#[cfg(target_os = "macos")]
mod iokit {
    use std::ffi::{CStr, CString, c_char, c_void};

    type Ref = *const c_void;
    const UTF8: u32 = 0x0800_0100;
    const SINT64: i32 = 4;

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingServices(port: u32, matching: Ref, iterator: *mut u32) -> i32;
        fn IOIteratorNext(iterator: u32) -> u32;
        fn IOObjectRelease(object: u32) -> i32;
        fn IORegistryEntryCreateCFProperty(entry: u32, key: Ref, alloc: Ref, options: u32) -> Ref;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(alloc: Ref, text: *const c_char, encoding: u32) -> Ref;
        fn CFRelease(object: Ref);
        fn CFGetTypeID(object: Ref) -> usize;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFDictionaryGetCount(dictionary: Ref) -> isize;
        fn CFDictionaryGetKeysAndValues(dictionary: Ref, keys: *mut Ref, values: *mut Ref);
        fn CFStringGetCString(text: Ref, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
        fn CFNumberGetValue(number: Ref, kind: i32, out: *mut c_void) -> u8;
    }

    struct Owned(Ref);
    impl Drop for Owned {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0) };
            }
        }
    }

    fn number(value: Ref) -> Option<i64> {
        let mut out = 0i64;
        (unsafe { CFGetTypeID(value) == CFNumberGetTypeID() }
            && unsafe { CFNumberGetValue(value, SINT64, (&mut out as *mut i64).cast()) } != 0)
            .then_some(out)
    }

    fn text(value: Ref) -> Option<String> {
        if unsafe { CFGetTypeID(value) != CFStringGetTypeID() } {
            return None;
        }
        let mut buffer = [0 as c_char; 256];
        (unsafe { CFStringGetCString(value, buffer.as_mut_ptr(), buffer.len() as isize, UTF8) }
            != 0)
            .then(|| {
                unsafe { CStr::from_ptr(buffer.as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            })
    }

    fn property(entry: u32, key: &str) -> Option<Owned> {
        let key = CString::new(key).ok()?;
        let key = Owned(unsafe { CFStringCreateWithCString(std::ptr::null(), key.as_ptr(), UTF8) });
        if key.0.is_null() {
            return None;
        }
        let value = unsafe { IORegistryEntryCreateCFProperty(entry, key.0, std::ptr::null(), 0) };
        (!value.is_null()).then_some(Owned(value))
    }

    pub fn accelerator_text() -> Option<String> {
        let mut iterator = 0u32;
        let matching = unsafe { IOServiceMatching(c"IOAccelerator".as_ptr()) };
        if matching.is_null()
            || unsafe { IOServiceGetMatchingServices(0, matching, &mut iterator) } != 0
        {
            return None;
        }
        let mut out = String::new();
        loop {
            let entry = unsafe { IOIteratorNext(iterator) };
            if entry == 0 {
                break;
            }
            if out.is_empty() {
                if let Some(cores) = property(entry, "gpu-core-count").and_then(|v| number(v.0)) {
                    out.push_str(&format!("  | \"gpu-core-count\" = {cores}\n"));
                }
                if let Some(stats) = property(entry, "PerformanceStatistics")
                    && unsafe { CFGetTypeID(stats.0) == CFDictionaryGetTypeID() }
                {
                    let count = unsafe { CFDictionaryGetCount(stats.0) }.max(0) as usize;
                    let mut keys = vec![std::ptr::null(); count];
                    let mut values = vec![std::ptr::null(); count];
                    unsafe {
                        CFDictionaryGetKeysAndValues(
                            stats.0,
                            keys.as_mut_ptr(),
                            values.as_mut_ptr(),
                        )
                    };
                    let fields: Vec<_> = keys
                        .iter()
                        .zip(&values)
                        .filter_map(|(key, value)| {
                            Some(format!("\"{}\"={}", text(*key)?, number(*value)?))
                        })
                        .collect();
                    out.push_str(&format!(
                        "  |   \"PerformanceStatistics\" = {{{}}}\n",
                        fields.join(",")
                    ));
                }
            }
            unsafe { IOObjectRelease(entry) };
        }
        unsafe { IOObjectRelease(iterator) };
        (!out.is_empty()).then_some(out)
    }
}

/// Asks Metal for the numbers `ioreg` does not expose: whether the device shares system memory and
/// how much of it the GPU may keep resident. Loaded once, off the sampling thread.
#[cfg(target_os = "macos")]
mod metal {
    use std::ffi::{c_char, c_void};
    use std::sync::OnceLock;

    #[derive(Clone, Copy, Debug)]
    pub struct Info {
        pub unified: bool,
        pub working_set_limit: u64,
    }

    #[link(name = "Metal", kind = "framework")]
    unsafe extern "C" {
        fn MTLCreateSystemDefaultDevice() -> *mut c_void;
    }
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_release(object: *mut c_void);
    }

    static INFO: OnceLock<Option<Info>> = OnceLock::new();

    fn query() -> Option<Info> {
        let device = unsafe { MTLCreateSystemDefaultDevice() };
        if device.is_null() {
            return None;
        }
        type SendU64 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64;
        type SendBool = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
        // objc_msgSend must be called through a pointer typed with the real return type.
        let (send_u64, send_bool) = unsafe {
            (
                std::mem::transmute::<unsafe extern "C" fn(), SendU64>(objc_msgSend),
                std::mem::transmute::<unsafe extern "C" fn(), SendBool>(objc_msgSend),
            )
        };
        let info = unsafe {
            Info {
                unified: send_bool(device, sel_registerName(c"hasUnifiedMemory".as_ptr())),
                working_set_limit: send_u64(
                    device,
                    sel_registerName(c"recommendedMaxWorkingSetSize".as_ptr()),
                ),
            }
        };
        unsafe { objc_release(device) };
        Some(info)
    }

    /// Blocks on first use; call from a background thread.
    pub fn load() -> Option<Info> {
        *INFO.get_or_init(query)
    }

    /// Never blocks; `None` until `load` has finished or when Metal is unavailable.
    pub fn cached() -> Option<Info> {
        INFO.get().copied().flatten()
    }
}

/// Apple counters are independent of optional system_profiler/powermetrics probes.
#[cfg(target_os = "macos")]
pub fn discover(
    config: &crate::config::Config,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut extra = std::collections::BTreeMap::new();
    if config.collect_gpu {
        let _ = metal::load();
        let value = match iokit::accelerator_text() {
            Some(output) => Ok(output),
            // Fall back to the tool if the registry cannot be queried directly.
            None => crate::discovery::command(
                "ioreg",
                &["-r", "-c", "IOAccelerator", "-l"],
                std::time::Duration::from_secs(3),
            ),
        }
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
            // Metal answers asynchronously on first use; assume Apple Silicon is unified until then.
            let metal = metal::cached();
            let unified = metal.map_or(cfg!(target_arch = "aarch64"), |info| info.unified);
            let limit = metal
                .filter(|info| info.unified && info.working_set_limit > 0)
                .map(|info| info.working_set_limit);
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
                unified_memory: unified,
                memory_limit: limit,
                process_status: "macOS does not report GPU memory or load per process".into(),
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn iokit_text_is_parseable_when_an_accelerator_exists() {
        // CI virtual machines may expose no accelerator; only validate the format when present.
        if let Some(text) = super::iokit::accelerator_text() {
            assert!(text.contains("\"PerformanceStatistics\" = {"), "{text}");
            assert!(text.contains("\"In use system memory\"="), "{text}");
        }
    }
}
