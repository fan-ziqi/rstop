use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::{
    fs,
    io::Read,
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
static DRM_FDS: std::sync::OnceLock<std::sync::RwLock<HashMap<u32, Vec<std::path::PathBuf>>>> =
    std::sync::OnceLock::new();
#[cfg(target_os = "linux")]
pub fn drm_fds() -> HashMap<u32, Vec<std::path::PathBuf>> {
    DRM_FDS
        .get_or_init(Default::default)
        .read()
        .map(|fds| fds.clone())
        .unwrap_or_default()
}
#[cfg(target_os = "linux")]
#[derive(Default)]
struct Owners {
    pids: HashMap<u64, Vec<u32>>,
    names: HashMap<u32, String>,
    complete: bool,
    unreadable: u64,
}
#[cfg(target_os = "linux")]
fn owner_scan() -> Owners {
    #[cfg(target_os = "linux")]
    {
        let began = Instant::now();
        let mut owners: HashMap<u64, Vec<u32>> = HashMap::new();
        let mut names = HashMap::<u32, String>::new();
        let mut drm = HashMap::<u32, Vec<std::path::PathBuf>>::new();
        let mut denied = 0;
        let mut complete = true;
        'outer: for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(files) = fs::read_dir(entry.path().join("fd")) else {
                denied += 1;
                continue;
            };
            if let Ok(name) = fs::read_to_string(entry.path().join("comm")) {
                names.insert(pid, name.trim().into());
            }
            for file in files.flatten() {
                if began.elapsed() > Duration::from_millis(700) {
                    complete = false;
                    break 'outer;
                }
                let Ok(link) = fs::read_link(file.path()) else {
                    continue;
                };
                if link.starts_with("/dev/dri") {
                    drm.entry(pid)
                        .or_default()
                        .push(std::path::PathBuf::from(format!(
                            "/proc/{pid}/fdinfo/{}",
                            file.file_name().to_string_lossy()
                        )));
                }
                let text = link.to_string_lossy();
                if let Some(inode) = text
                    .strip_prefix("socket:[")
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    let pids = owners.entry(inode).or_default();
                    if !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
        }
        if let Ok(mut cache) = DRM_FDS.get_or_init(Default::default).write() {
            *cache = drm;
        }
        Owners {
            pids: owners,
            names,
            complete,
            unreadable: denied,
        }
    }
}
pub fn decode_address(address: &str) -> String {
    let Some((ip, port)) = address.split_once(':') else {
        return address.into();
    };
    let port = u16::from_str_radix(port, 16).unwrap_or(0);
    if ip.len() == 8
        && let Ok(ip) = u32::from_str_radix(ip, 16)
    {
        return format!("{}:{port}", std::net::Ipv4Addr::from(ip.to_le_bytes()));
    }
    if ip.len() == 32 {
        let mut bytes = [0u8; 16];
        for (i, part) in ip.as_bytes().as_chunks::<8>().0.iter().enumerate() {
            let Ok(text) = std::str::from_utf8(part) else {
                return address.into();
            };
            let Ok(value) = u32::from_str_radix(text, 16) else {
                return address.into();
            };
            bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        return format!("[{}]:{port}", std::net::Ipv6Addr::from(bytes));
    }
    address.into()
}

/// Socket counters are TCP payload counters, not interface/wire traffic.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub tags: Vec<crate::tags::Tag>,
    pub id: String,
    pub protocol: String,
    pub local: String,
    pub remote: String,
    pub state: String,
    pub inode: u64,
    pub pids: Vec<u32>,
    pub process_names: Vec<String>,
    pub receive_queue: Option<u64>,
    pub send_queue: Option<u64>,
    pub received: Option<u64>,
    pub sent: Option<u64>,
    pub rx_per_sec: Option<f64>,
    pub tx_per_sec: Option<f64>,
    pub rtt_ms: Option<f64>,
    pub retransmits: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sample {
    pub sampled_at: u64,
    pub owners_sampled_at: u64,
    pub status: String,
    pub notes: Vec<String>,
    pub owner_scan_complete: bool,
    pub unreadable_processes: u64,
    pub truncated: bool,
    pub connections: Vec<Connection>,
}

#[derive(Default)]
pub struct Collector {
    previous: HashMap<String, (Option<u64>, Option<u64>)>,
    at: Option<std::time::Instant>,
    #[cfg(target_os = "linux")]
    owners_at: Option<std::time::Instant>,
    #[cfg(target_os = "linux")]
    owners: Owners,
    #[cfg(target_os = "linux")]
    owners_sampled_at: u64,
}
impl Collector {
    pub fn collect(&mut self) -> Sample {
        self.collect_with_sockets(true)
    }
    pub fn collect_with_sockets(&mut self, sockets: bool) -> Sample {
        let mut sample = Sample {
            sampled_at: crate::collector::unix_time(),
            ..Default::default()
        };
        #[cfg(target_os = "linux")]
        {
            if self.owners_at.is_none_or(|at| at.elapsed().as_secs() >= 5) {
                self.owners = owner_scan();
                self.owners_at = Some(Instant::now());
                self.owners_sampled_at = sample.sampled_at;
            }
            sample.owners_sampled_at = self.owners_sampled_at;
            sample.owner_scan_complete = self.owners.complete;
            sample.unreadable_processes = self.owners.unreadable;
            if !sockets {
                sample.status = "Loading connection details…".into();
                return sample;
            }
            let native = match crate::inet_diag::collect() {
                Ok(rows) => {
                    sample.connections = rows;
                    true
                }
                Err(error) => {
                    sample
                        .notes
                        .push(format!("inet_diag unavailable; using procfs/ss: {error}"));
                    false
                }
            };
            if !native {
                for protocol in ["tcp", "tcp6", "udp", "udp6"] {
                    match fs::File::open(format!("/proc/net/{protocol}")).and_then(|file| {
                        let mut text = String::new();
                        file.take(16 * 1024 * 1024).read_to_string(&mut text)?;
                        Ok(text)
                    }) {
                        Ok(text) => {
                            if text.len() >= 16 * 1024 * 1024 {
                                sample.truncated = true;
                            }
                            sample.connections.extend(parse_proc(protocol, &text));
                        }
                        Err(error) => sample.notes.push(format!("{protocol}: {error}")),
                    }
                }
            }
            sample.status = if sample.notes.is_empty() {
                "Current network namespace"
            } else {
                "Partial socket sample"
            }
            .into();
            for row in &mut sample.connections {
                row.pids = self
                    .owners
                    .pids
                    .get(&row.inode)
                    .cloned()
                    .unwrap_or_default();
                row.process_names = row
                    .pids
                    .iter()
                    .filter_map(|pid| self.owners.names.get(pid).cloned())
                    .collect();
            }
            // -O keeps TCP_INFO with its socket; -e exposes inode and socket cookie.
            if !native {
                match crate::discovery::command("ss", &["-H", "-O", "-t", "-a", "-n", "-i", "-e"], Duration::from_millis(1500)) {
                Ok(text) => apply_tcp_info(&mut sample.connections, &text),
                Err(error) => sample.notes.push(format!("TCP rates unavailable: {error}. Install iproute2 (ss); UDP byte counters remain unavailable.")),
            }
            }
        }
        #[cfg(target_os = "windows")]
        {
            if !sockets {
                return sample;
            }
            match crate::discovery::command("netstat", &["-ano"], std::time::Duration::from_secs(2))
            {
                Ok(text) => {
                    sample.connections = parse_netstat(&text);
                    sample.status = "netstat · TCP/UDP · byte counters unavailable".into();
                    sample.owner_scan_complete = true;
                    sample.owners_sampled_at = sample.sampled_at;
                }
                Err(error) => sample.status = format!("Connection query failed: {error}"),
            }
        }
        #[cfg(target_os = "macos")]
        {
            if !sockets {
                return sample;
            }
            match crate::discovery::command(
                "lsof",
                &["-nP", "-i", "-FpcfdnPtT"],
                std::time::Duration::from_secs(2),
            ) {
                Ok(text) => {
                    sample.connections = parse_lsof(&text);
                    sample.status = "lsof · readable processes · byte counters unavailable".into();
                    sample.owners_sampled_at = sample.sampled_at;
                    sample.owner_scan_complete = true;
                }
                Err(error) => sample.status = format!("Connection query failed: {error}"),
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            sample.status = "Connection monitoring unavailable on this platform".into();
        }
        if sample.connections.len() > 20_000 {
            sample.connections.truncate(20_000);
            sample.truncated = true;
        }
        self.update_rates(&mut sample, std::time::Instant::now());
        sample
    }
    pub fn update_rates(&mut self, sample: &mut Sample, now: std::time::Instant) {
        let seconds = self
            .at
            .map(|at| now.duration_since(at).as_secs_f64())
            .filter(|s| *s > 0.);
        let mut next = HashMap::new();
        for row in &mut sample.connections {
            row.rx_per_sec = None;
            row.tx_per_sec = None;
            if let Some((old_rx, old_tx)) = self.previous.get(&row.id) {
                let rate = |old: Option<u64>, current: Option<u64>| {
                    old.zip(current)
                        .and_then(|(old, current)| current.checked_sub(old))
                        .zip(seconds)
                        .map(|(delta, seconds)| delta as f64 / seconds)
                };
                row.rx_per_sec = rate(*old_rx, row.received);
                row.tx_per_sec = rate(*old_tx, row.sent);
            }
            next.insert(row.id.clone(), (row.received, row.sent));
        }
        self.previous = next;
        self.at = Some(now);
    }
}

pub fn scan() -> Value {
    serde_json::to_value(Collector::default().collect()).unwrap_or_default()
}

/// Independent from slow container/service discovery and the fast host sampler.
pub fn worker() -> (
    std::sync::Arc<std::sync::RwLock<Sample>>,
    std::thread::JoinHandle<()>,
) {
    worker_with_demand(|| Demand {
        owners: true,
        sockets: true,
    })
}
#[derive(Clone, Copy)]
pub struct Demand {
    pub owners: bool,
    pub sockets: bool,
}
pub fn worker_with_demand(
    demand: impl Fn() -> Demand + Send + 'static,
) -> (
    std::sync::Arc<std::sync::RwLock<Sample>>,
    std::thread::JoinHandle<()>,
) {
    let cache = std::sync::Arc::new(std::sync::RwLock::new(Sample::default()));
    let target = cache.clone();
    let worker = std::thread::spawn(move || {
        let mut collector = Collector::default();
        while !crate::STOP.load(std::sync::atomic::Ordering::Relaxed) {
            let needed = demand();
            if !needed.owners && !needed.sockets {
                std::thread::park_timeout(std::time::Duration::from_secs(1));
                continue;
            }
            let began = std::time::Instant::now();
            let value = collector.collect_with_sockets(needed.sockets);
            if let Ok(mut sample) = target.write() {
                *sample = value;
            }
            while began.elapsed()
                < std::time::Duration::from_secs(if needed.sockets { 1 } else { 5 })
                && demand().sockets == needed.sockets
                && !crate::STOP.load(std::sync::atomic::Ordering::Relaxed)
            {
                std::thread::park_timeout(std::time::Duration::from_millis(50));
            }
        }
    });
    (cache, worker)
}

pub fn parse_proc(protocol: &str, text: &str) -> Vec<Connection> {
    text.lines()
        .skip(1)
        .take(20_001)
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() < 10 {
                return None;
            }
            let inode = f[9].parse::<u64>().ok()?;
            let local = decode_address(f[1]);
            let remote = decode_address(f[2]);
            let state = match f[3] {
                "01" => "ESTABLISHED",
                "02" => "SYN_SENT",
                "03" => "SYN_RECV",
                "04" => "FIN_WAIT1",
                "05" => "FIN_WAIT2",
                "06" => "TIME_WAIT",
                "07" if protocol.starts_with("udp") => "BOUND",
                "07" => "CLOSED",
                "08" => "CLOSE_WAIT",
                "09" => "LAST_ACK",
                "0A" => "LISTEN",
                "0B" => "CLOSING",
                _ => "UNKNOWN",
            };
            let (tx, rx) = f[4].split_once(':')?;
            Some(Connection {
                id: format!("{protocol}:{inode}:{local}:{remote}"),
                protocol: protocol.into(),
                local,
                remote,
                state: state.into(),
                inode,
                receive_queue: u64::from_str_radix(rx, 16).ok(),
                send_queue: u64::from_str_radix(tx, 16).ok(),
                ..Default::default()
            })
        })
        .collect()
}

pub fn apply_tcp_info(rows: &mut [Connection], text: &str) {
    let mut data = HashMap::new();
    for line in text.lines() {
        let fields: HashMap<_, _> = line
            .split_whitespace()
            .filter_map(|v| v.split_once(':'))
            .collect();
        if let Some(inode) = fields.get("ino").and_then(|v| v.parse::<u64>().ok()) {
            data.insert(inode, fields);
        }
    }
    for row in rows.iter_mut().filter(|r| r.protocol.starts_with("tcp")) {
        let Some(fields) = data.get(&row.inode) else {
            continue;
        };
        if let Some(cookie) = fields.get("sk") {
            row.id.push_str(&format!(":{cookie}"));
        }
        row.received = fields.get("bytes_received").and_then(|v| v.parse().ok());
        row.sent = fields.get("bytes_sent").and_then(|v| v.parse().ok());
        row.rtt_ms = fields
            .get("rtt")
            .and_then(|v| v.split('/').next()?.parse().ok());
        row.retransmits = fields
            .get("retrans")
            .and_then(|v| v.split('/').nth(1)?.parse().ok());
    }
}

pub fn parse_netstat(text: &str) -> Vec<Connection> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            let (state, pid) = match f.first().copied()? {
                "TCP" if f.len() == 5 => (f[3], f[4]),
                "UDP" if f.len() == 4 => ("BOUND", f[3]),
                _ => return None,
            };
            Some(Connection {
                id: format!("{}:{}:{}:{pid}", f[0], f[1], f[2]),
                protocol: f[0].to_lowercase(),
                local: f[1].into(),
                remote: f[2].into(),
                state: state.into(),
                pids: vec![pid.parse().ok()?],
                ..Default::default()
            })
        })
        .collect()
}
pub fn parse_lsof(text: &str) -> Vec<Connection> {
    fn finish(
        rows: &mut Vec<Connection>,
        row: &mut Connection,
        pid: u32,
        name: &str,
        fd: &str,
        socket: &str,
    ) {
        let mut row = std::mem::take(row);
        if row.protocol.is_empty() || row.local.is_empty() {
            return;
        }
        // Darwin's device character field identifies the socket PCB, shared by dup'd fds.
        // Keep descriptors separate when that identity is unavailable.
        let identity = if !socket.is_empty() && socket != "0x0" {
            format!("socket:{socket}")
        } else if !fd.is_empty() {
            format!("fd:{fd}")
        } else {
            format!("row:{}", rows.len())
        };
        row.id = format!(
            "{pid}:{}:{}->{}:{identity}",
            row.protocol, row.local, row.remote
        );
        row.pids = vec![pid];
        row.process_names = vec![name.into()];
        rows.push(row);
    }

    let mut result = vec![];
    let (mut pid, mut name) = (0u32, "");
    let (mut fd, mut socket) = ("", "");
    let mut row = Connection::default();
    for line in text.lines() {
        let Some((tag, value)) = line.split_at_checked(1) else {
            continue;
        };
        match tag {
            "p" | "f" => {
                finish(&mut result, &mut row, pid, name, fd, socket);
                socket = "";
                if tag == "p" {
                    pid = value.parse().unwrap_or(0);
                    name = "";
                    fd = "";
                } else {
                    fd = value;
                }
            }
            "c" => name = value,
            "d" => socket = value,
            "P" => row.protocol = value.to_lowercase(),
            "n" => {
                let (local, remote) = value.split_once("->").unwrap_or((value, "*:*"));
                row.local = local.into();
                row.remote = remote.into();
            }
            "T" => {
                if let Some(state) = value.strip_prefix("ST=") {
                    row.state = state.into();
                } else if let Some(bytes) = value.strip_prefix("QR=") {
                    row.receive_queue = bytes.parse().ok();
                } else if let Some(bytes) = value.strip_prefix("QS=") {
                    row.send_queue = bytes.parse().ok();
                }
            }
            _ => {}
        }
    }
    finish(&mut result, &mut row, pid, name, fd, socket);
    for row in &mut result {
        if row.state.is_empty() {
            row.state = "BOUND".into();
        }
    }
    // A socket duplicated across several descriptors of one process is listed once per fd.
    let mut seen = std::collections::HashSet::new();
    result.retain(|row| seen.insert(row.id.clone()));
    result
}

pub fn state_help(state: &str) -> &'static str {
    match state {
        "ESTABLISHED" => "Connected. Idle connections can legitimately have zero traffic.",
        "LISTEN" | "BOUND" => {
            "Waiting for incoming traffic. Wildcard addresses accept traffic on all local interfaces."
        }
        "SYN_SENT" => {
            "Waiting for the peer to accept TCP. Check the destination, route and firewall; this alone does not identify the cause."
        }
        "CLOSE_WAIT" => {
            "The peer closed. If many accumulate, inspect whether this application closes its sockets."
        }
        "TIME_WAIT" => "Recently closed; the OS temporarily retains it. This is normally expected.",
        _ => {
            "Connection is opening or closing. Compare successive samples before diagnosing a fault."
        }
    }
}
