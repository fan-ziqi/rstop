//! Slow optional system and local-application probes.
#[cfg(target_os = "linux")]
use super::{parse_process_fields, read};
use crate::{
    collector::unix_time,
    config::Config,
    discovery::{command, command_with_stderr},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};
pub fn discover(config: &Config) -> BTreeMap<String, Value> {
    let mut extra = BTreeMap::new();
    #[cfg(target_os = "linux")]
    {
        let mut nodes = vec![];
        for entry in fs::read_dir("/sys/devices/system/node")
            .into_iter()
            .flatten()
            .flatten()
        {
            if !entry
                .file_name()
                .to_string_lossy()
                .strip_prefix("node")
                .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }
            nodes.push(json!({"node":entry.file_name().to_string_lossy(),"cpus":read(entry.path().join("cpulist")),"memory":read(entry.path().join("meminfo")),"numastat":read(entry.path().join("numastat"))}));
        }
        if !nodes.is_empty() {
            extra.insert("NUMA".into(), json!(nodes));
        }
        for (name, path) in [
            ("Interrupts", "/proc/interrupts"),
            ("Soft IRQ", "/proc/softirqs"),
            ("Software RAID", "/proc/mdstat"),
            ("Memory details", "/proc/meminfo"),
            ("VM counters", "/proc/vmstat"),
        ] {
            if let Some(value) = read(path)
                && !value.is_empty()
            {
                extra.insert(
                    name.into(),
                    if name == "Memory details" {
                        json!(parse_process_fields(&value))
                    } else {
                        json!(value)
                    },
                );
            }
        }
        let mut sensors = vec![];
        for device in fs::read_dir("/sys/class/hwmon")
            .into_iter()
            .flatten()
            .flatten()
        {
            for entry in fs::read_dir(device.path()).into_iter().flatten().flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                if !file.ends_with("_input") {
                    continue;
                }
                let (unit, divisor) = if file.starts_with("fan") {
                    ("RPM", 1.0)
                } else if file.starts_with("in") {
                    ("V", 1000.0)
                } else if file.starts_with("power") {
                    ("W", 1e6)
                } else if file.starts_with("curr") {
                    ("A", 1000.0)
                } else {
                    continue;
                };
                let Some(value) = read(entry.path()).and_then(|s| s.parse::<f64>().ok()) else {
                    continue;
                };
                let label = read(
                    entry
                        .path()
                        .with_file_name(file.replace("_input", "_label")),
                )
                .unwrap_or(file);
                sensors.push(json!({"device":read(device.path().join("name")),"label":label,"value":value/divisor,"unit":unit}));
            }
        }
        if !sensors.is_empty() {
            extra.insert("Hardware sensors".into(), json!(sensors));
        }
        if let Ok(output) = command("iw", &["dev"], Duration::from_secs(2)) {
            let interfaces: Vec<_> = output
                .lines()
                .filter_map(|l| l.trim().strip_prefix("Interface "))
                .map(ToString::to_string)
                .collect();
            for interface in interfaces {
                probe(
                    &mut extra,
                    &format!("Wi-Fi {interface}"),
                    "iw",
                    &["dev", &interface, "link"],
                    2,
                );
            }
        }
        if config.process_network {
            let script = "kprobe:tcp_recvmsg { @peek[tid] = arg3 & 2; } kretprobe:tcp_sendmsg /retval > 0/ { @tx[pid] = sum(retval); } kretprobe:tcp_recvmsg /retval > 0 && !@peek[tid]/ { @rx[pid] = sum(retval); delete(@peek[tid]); } interval:s:1 { print(@tx); print(@rx); clear(@tx); clear(@rx); clear(@peek); exit(); } END { clear(@tx); clear(@rx); clear(@peek); }";
            probe(
                &mut extra,
                "Process TCP bytes / 1s (bpftrace)",
                "bpftrace",
                &["-q", "-f", "json", "-e", script],
                8,
            );
        }
        if let Ok(output) = command(
            "smartctl",
            &["--scan-open", "--json"],
            Duration::from_secs(3),
        ) && let Ok(value) = serde_json::from_str::<Value>(&output)
            && let Some(devices) = value["devices"].as_array()
        {
            for device in devices.iter().take(32) {
                if let Some(name) = device["name"].as_str() {
                    let result = crate::discovery::command_partial(
                        "smartctl",
                        &["-a", "--json", name],
                        Duration::from_secs(3),
                    );
                    extra.insert(
                        format!("SMART {name}"),
                        result
                            .map(|s| serde_json::from_str(&s).unwrap_or(json!(s)))
                            .unwrap_or_else(|e| json!({"error":e.to_string()})),
                    );
                }
            }
        }
        probe(
            &mut extra,
            "Local virtual machines",
            "virsh",
            &[
                "--connect",
                "qemu:///system",
                "domstats",
                "--all",
                "--state",
                "--cpu-total",
                "--balloon",
                "--vcpu",
                "--block",
                "--interface",
            ],
            3,
        );
        probe(&mut extra, "Local LXC", "lxc-ls", &["--fancy"], 3);
        probe(
            &mut extra,
            "Local LXD",
            "lxc",
            &["list", "local:", "--format=json"],
            3,
        );
        if config.collect_gpu {
            probe(&mut extra, "Ascend NPU", "npu-smi", &["info"], 3);
        }
        if let Ok(entries) = fs::read_dir("/sys/class/accel") {
            let cards:Vec<_>=entries.flatten().map(|e|json!({"name":e.file_name().to_string_lossy(),"device":fs::canonicalize(e.path().join("device")).ok(),"uevent":read(e.path().join("device/uevent"))})).collect();
            if !cards.is_empty() {
                extra.insert("Accelerators (sysfs)".into(), json!(cards));
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        probe(
            &mut extra,
            "CPU state / kernel counters",
            "sysctl",
            &["kern.cp_time", "vm.swapusage", "hw.memsize"],
            2,
        );
        probe(&mut extra, "Memory pages", "vm_stat", &[], 2);
        probe(
            &mut extra,
            "Disk I/O details",
            "iostat",
            &["-d", "-w", "1", "-c", "2"],
            3,
        );
        probe(&mut extra, "Interfaces and errors", "netstat", &["-ib"], 2);
        probe(
            &mut extra,
            "Connections and owners",
            "lsof",
            &["-nP", "-i", "-FpcnT"],
            3,
        );
        if config.collect_gpu {
            probe(
                &mut extra,
                "Apple GPU / Neural Engine",
                "ioreg",
                &["-r", "-c", "IOAccelerator", "-l"],
                3,
            );
        }
        probe(
            &mut extra,
            "Apple power / thermal",
            "powermetrics",
            &[
                "--samplers",
                "cpu_power,gpu_power,ane_power,smc",
                "-n",
                "1",
                "-i",
                "1000",
            ],
            3,
        );
        probe(
            &mut extra,
            "Wi-Fi",
            "system_profiler",
            &["SPAirPortDataType", "-json", "-detailLevel", "mini"],
            4,
        );
        probe(
            &mut extra,
            "Local virtual machines",
            "virsh",
            &["--connect", "qemu:///session", "domstats", "--all"],
            3,
        );
    }
    #[cfg(target_os = "windows")]
    {
        if config.collect_gpu {
            extra.insert(
                "Windows adapters".into(),
                crate::windows_inspect::adapters(),
            );
        }
        let commands = [
            (
                "CPU state / DPC / interrupts",
                "Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor | Select Name,PercentUserTime,PercentPrivilegedTime,PercentInterruptTime,PercentDPCTime,InterruptsPersec | ConvertTo-Json -Compress",
            ),
            (
                "Memory / cache / paging",
                "Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory | Select AvailableBytes,CacheBytes,PagesPersec,PageReadsPersec,PageWritesPersec | ConvertTo-Json -Compress",
            ),
            (
                "Disk latency / IOPS",
                "Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk | Select Name,DiskReadsPersec,DiskWritesPersec,AvgDisksecPerRead,AvgDisksecPerWrite,CurrentDiskQueueLength,PercentDiskTime | ConvertTo-Json -Compress",
            ),
            (
                "Connections / owners",
                "@(Get-NetTCPConnection | Select LocalAddress,LocalPort,RemoteAddress,RemotePort,State,OwningProcess) | ConvertTo-Json -Compress",
            ),
            ("Wi-Fi", "netsh wlan show interfaces"),
            (
                "Hardware sensors",
                "Get-CimInstance -Namespace root/LibreHardwareMonitor -ClassName Sensor | Select Name,SensorType,Value,Min,Max | ConvertTo-Json -Compress",
            ),
            (
                "Hyper-V virtual machines",
                "Get-VM | Select Name,State,CPUUsage,MemoryAssigned,Uptime | ConvertTo-Json -Compress",
            ),
            (
                "GPU process memory",
                "Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory | Select Name,DedicatedUsage,SharedUsage | ConvertTo-Json -Compress",
            ),
            (
                "GPU engines and memory",
                "Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine | Select Name,UtilizationPercentage | ConvertTo-Json -Compress",
            ),
        ];
        for (name, script) in commands {
            if name.starts_with("GPU") && !config.collect_gpu {
                continue;
            }
            probe(
                &mut extra,
                name,
                "powershell.exe",
                &["-NoProfile", "-NonInteractive", "-Command", script],
                4,
            );
        }
    }
    // Kubernetes is queried ONLY for an explicitly loopback API server. No remote context is contacted.
    if let Ok(output) = command(
        "kubectl",
        &["config", "view", "--minify", "-o", "json"],
        Duration::from_secs(2),
    ) && let Ok(value) = serde_json::from_str::<Value>(&output)
        && let Some(server) = value["clusters"][0]["cluster"]["server"].as_str()
    {
        let active_auth = value["users"].as_array().is_some_and(|users| {
            users
                .iter()
                .any(|u| !u["user"]["exec"].is_null() || !u["user"]["auth-provider"].is_null())
        });
        let proxy = value["clusters"][0]["cluster"]["proxy-url"]
            .as_str()
            .is_some_and(|s| !s.is_empty());
        if local_http(server) && !active_auth && !proxy {
            probe(
                &mut extra,
                "Local Kubernetes pods",
                "kubectl",
                &[
                    &format!("--server={server}"),
                    "--request-timeout=2s",
                    "get",
                    "pods",
                    "--all-namespaces",
                    "-o",
                    "json",
                ],
                3,
            );
            probe(
                &mut extra,
                "Local Kubernetes resources",
                "kubectl",
                &[
                    &format!("--server={server}"),
                    "--request-timeout=2s",
                    "top",
                    "pods",
                    "--all-namespaces",
                ],
                3,
            );
        } else {
            extra.insert(
                "Kubernetes discovery".into(),
                json!({"status":"nonlocal or external-auth/proxy context skipped"}),
            );
        }
    }
    for path in &config.monitored_folders {
        let result = folder_size(path, 100_000, Duration::from_millis(400));
        extra.insert(format!("Folder {}", path.display()), json!(result));
    }
    for p in config.probes.iter().filter(|_| config.config_exec) {
        let args: Vec<_> = p.command.iter().skip(1).map(String::as_str).collect();
        probe(
            &mut extra,
            &format!("App: {}", p.name),
            &p.command[0],
            &args,
            p.timeout_seconds,
        );
    }
    extra
}
fn probe(
    extra: &mut BTreeMap<String, Value>,
    name: &str,
    program: &str,
    args: &[&str],
    seconds: u64,
) {
    match command_with_stderr(program, args, Duration::from_secs(seconds)) {
        Ok(output) if !output.trim().is_empty() => {
            extra.insert(
                name.into(),
                serde_json::from_str(&output)
                    .unwrap_or(json!({"output":output,"sampled_at":unix_time()})),
            );
        }
        Err(error) if !error.to_string().contains("not installed") => {
            extra.insert(name.into(), json!({"error":error.to_string()}));
        }
        _ => {}
    }
}
pub fn local_http(server: &str) -> bool {
    let Some(rest) = server
        .strip_prefix("https://")
        .or_else(|| server.strip_prefix("http://"))
    else {
        return false;
    };
    if rest.contains('@') || rest.contains('\\') || rest.chars().any(char::is_control) {
        return false;
    }
    crate::discovery::is_local_endpoint(&format!(
        "tcp://{}",
        rest.split(['/', '?', '#']).next().unwrap_or_default()
    ))
}

pub fn folder_size(path: &Path, max_entries: usize, budget: Duration) -> Value {
    let now = Instant::now();
    let mut todo = vec![path.to_path_buf()];
    let mut count = 0;
    let mut bytes = 0;
    let mut errors = 0;
    let mut complete = true;
    while let Some(path) = todo.pop() {
        if count >= max_entries || now.elapsed() > budget {
            complete = false;
            break;
        }
        count += 1;
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_symlink() => {}
            Ok(meta) if meta.is_dir() => match fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        if count + todo.len() >= max_entries || now.elapsed() > budget {
                            complete = false;
                            break;
                        }
                        match entry {
                            Ok(entry) => todo.push(entry.path()),
                            Err(_) => errors += 1,
                        }
                    }
                }
                Err(_) => errors += 1,
            },
            Ok(meta) => bytes += meta.len(),
            Err(_) => errors += 1,
        }
    }
    json!({"bytes":bytes,"entries":count,"complete":complete&&errors==0,"errors":errors,"symlinks":"not followed"})
}
