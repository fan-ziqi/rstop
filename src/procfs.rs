//! Linux's fast process sample. Cached descriptors use pread, so a tick needs no seeks.
use crate::model::Process;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, Read},
    os::unix::fs::FileExt,
};

#[derive(Default)]
struct Files {
    stat: Option<File>,
    memory: Option<File>,
    io: Option<File>,
}
struct Cached {
    files: Files,
    start: Option<u64>,
    cpu: u64,
    read: u64,
    written: u64,
    name: String,
    command: String,
    uid: Option<u32>,
}
pub(crate) struct Processes {
    cache: HashMap<u32, Cached>,
    ticks: u64,
    page: u64,
    boot: u64,
    cache_limit: usize,
}
impl Default for Processes {
    fn default() -> Self {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        unsafe {
            libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit);
        }
        Self {
            cache: HashMap::new(),
            ticks: unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64,
            page: unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1) as u64,
            boot: sysinfo::System::boot_time(),
            cache_limit: ((limit.rlim_cur / 6) as usize).min(4096),
        }
    }
}
fn read(file: &mut Option<File>, path: &str, keep: bool, buffer: &mut [u8]) -> io::Result<usize> {
    if let Some(cached) = file {
        if let Ok(count) = cached.read_at(buffer, 0)
            && count > 0
        {
            return Ok(count);
        }
        *file = None;
    }
    let opened = File::open(path)?;
    let count = opened.read_at(buffer, 0)?;
    if keep {
        *file = Some(opened);
    }
    Ok(count)
}
struct Stat<'a> {
    name: &'a str,
    fields: [&'a str; 50],
}
impl<'a> Stat<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let begin = text.find('(')?;
        let end = text.rfind(')')?;
        if end <= begin {
            return None;
        }
        let mut fields = [""; 50];
        let mut count = 0;
        for (i, value) in text[end + 1..]
            .split_ascii_whitespace()
            .take(50)
            .enumerate()
        {
            fields[i] = value;
            count += 1;
        }
        (count >= 22).then_some(Self {
            name: &text[begin + 1..end],
            fields,
        })
    }
    fn n(&self, i: usize) -> u64 {
        self.fields[i].parse().unwrap_or(0)
    }
}
fn io_counters(text: &str) -> (u64, u64) {
    let mut counters = (0, 0);
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("read_bytes:") {
            counters.0 = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = line.strip_prefix("write_bytes:") {
            counters.1 = v.trim().parse().unwrap_or(0);
        }
    }
    counters
}
impl Processes {
    pub fn collect(&mut self, users: &sysinfo::Users, seconds: f64) -> Vec<Process> {
        let names: HashMap<_, _> = users.iter().map(|u| (**u.id(), u.name())).collect();
        let mut seen = HashSet::with_capacity(self.cache.len());
        let mut result = Vec::with_capacity(self.cache.len());
        let mut stat_buffer = [0u8; 8192];
        let mut buffer = [0u8; 4096];
        for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let keep = self.cache.len() < self.cache_limit;
            let cached = self.cache.entry(pid).or_insert_with(|| Cached {
                files: Files::default(),
                start: None,
                cpu: 0,
                read: 0,
                written: 0,
                name: String::new(),
                command: String::new(),
                uid: None,
            });
            let path = format!("/proc/{pid}");
            let count = match read(
                &mut cached.files.stat,
                &format!("{path}/stat"),
                keep,
                &mut stat_buffer,
            ) {
                Ok(n) if n > 0 => n,
                _ => {
                    cached.files = Files::default();
                    continue;
                }
            };
            let text = String::from_utf8_lossy(&stat_buffer[..count]);
            let Some(stat) = Stat::parse(&text) else {
                continue;
            };
            let start = stat.n(19);
            let fresh = cached.start != Some(start);
            if fresh || cached.name != stat.name {
                if fresh {
                    cached.files.memory = None;
                    cached.files.io = None;
                }
                cached.name = stat.name.into();
                cached.command = File::open(format!("{path}/cmdline"))
                    .and_then(|file| {
                        let mut bytes = Vec::new();
                        file.take(8 * 1024 * 1024).read_to_end(&mut bytes)?;
                        Ok(bytes)
                    })
                    .map(|bytes| {
                        String::from_utf8_lossy(&bytes)
                            .trim_end_matches('\0')
                            .replace('\0', " ")
                            .chars()
                            .filter(|c| !c.is_control())
                            .collect()
                    })
                    .unwrap_or_default();
                cached.uid = fs::read_to_string(format!("{path}/status"))
                    .ok()
                    .and_then(|text| {
                        text.lines().find_map(|l| {
                            l.strip_prefix("Uid:")?
                                .split_whitespace()
                                .next()?
                                .parse()
                                .ok()
                        })
                    });
            }
            let total_cpu = stat.n(11).saturating_add(stat.n(12));
            let cpu = if fresh {
                0.
            } else {
                total_cpu.saturating_sub(cached.cpu) as f64 * 100.
                    / self.ticks as f64
                    / seconds.max(0.001)
            };
            let (virtual_memory, memory) = read(
                &mut cached.files.memory,
                &format!("{path}/statm"),
                keep,
                &mut buffer,
            )
            .ok()
            .and_then(|n| {
                let text = std::str::from_utf8(&buffer[..n]).ok()?;
                let mut fields = text.split_ascii_whitespace();
                Some((
                    fields
                        .next()?
                        .parse::<u64>()
                        .ok()?
                        .saturating_mul(self.page),
                    fields
                        .next()?
                        .parse::<u64>()
                        .ok()?
                        .saturating_mul(self.page),
                ))
            })
            .unwrap_or((stat.n(20), stat.n(21).saturating_mul(self.page)));
            let (read_total, write_total) = read(
                &mut cached.files.io,
                &format!("{path}/io"),
                keep,
                &mut buffer,
            )
            .ok()
            .and_then(|n| std::str::from_utf8(&buffer[..n]).ok())
            .map(io_counters)
            .unwrap_or((0, 0));
            let state = match stat.fields[0] {
                "R" => "Runnable",
                "S" => "Sleeping",
                "D" => "UninterruptibleDiskSleep",
                "Z" => "Zombie",
                "T" => "Stopped",
                "t" => "Tracing",
                "X" | "x" => "Dead",
                "I" => "Idle",
                "P" => "Parked",
                _ => "Unknown",
            };
            result.push(Process {
                pid,
                parent: stat.fields[1].parse().ok(),
                start_time: self.boot + start / self.ticks,
                name: cached.name.clone(),
                command: cached.command.clone(),
                user: cached
                    .uid
                    .map(|id| {
                        names
                            .get(&id)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| id.to_string())
                    })
                    .unwrap_or_else(|| "—".into()),
                state: state.into(),
                cpu: cpu as f32,
                memory,
                virtual_memory,
                cpu_time_ms: total_cpu.saturating_mul(1000) / self.ticks,
                read_per_sec: if fresh {
                    0.
                } else {
                    read_total.saturating_sub(cached.read) as f64 / seconds.max(0.001)
                },
                write_per_sec: if fresh {
                    0.
                } else {
                    write_total.saturating_sub(cached.written) as f64 / seconds.max(0.001)
                },
                read_total,
                write_total,
                nice: stat.fields[16].parse().ok(),
                kernel_thread: stat.n(6) & 0x200000 != 0,
                runtime: crate::collector::unix_time()
                    .saturating_sub(self.boot + start / self.ticks),
                ..Default::default()
            });
            cached.start = Some(start);
            cached.cpu = total_cpu;
            cached.read = read_total;
            cached.written = write_total;
            seen.insert(pid);
        }
        self.cache.retain(|pid, _| seen.contains(pid));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comm_with_spaces_and_parentheses_does_not_shift_numeric_fields() {
        let text =
            "42 (a ) tricky (name) R 1 2 3 4 5 2097152 7 8 9 10 120 30 0 0 20 -5 2 0 999 4096 8";
        let stat = Stat::parse(text).unwrap();
        assert_eq!(stat.name, "a ) tricky (name");
        assert_eq!(stat.n(11) + stat.n(12), 150);
        assert_eq!(stat.fields[16], "-5");
        assert_eq!(stat.n(19), 999);
        assert!(Stat::parse("42 broken").is_none());
    }
    #[test]
    fn native_sample_keeps_identity_and_matches_owned_memory_and_io() {
        let mut collector = Processes::default();
        let users = sysinfo::Users::new_with_refreshed_list();
        let first = collector.collect(&users, 1.);
        let current = first.iter().find(|p| p.pid == std::process::id()).unwrap();
        assert!(current.memory > 0);
        assert!(!current.command.is_empty());
        let identity = current.start_time;
        let id = sysinfo::Pid::from_u32(std::process::id());
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[id]),
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        assert_eq!(identity, system.process(id).unwrap().start_time());
        let next = collector.collect(&users, 1.);
        assert_eq!(
            next.iter()
                .find(|p| p.pid == std::process::id())
                .unwrap()
                .start_time,
            identity
        );
    }

    #[test]
    fn cpu_delta_tracks_an_owned_busy_child_and_exited_processes_are_pruned() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "while :; do :; done"])
            .spawn()
            .unwrap();
        let pid = child.id();
        let mut collector = Processes::default();
        let users = sysinfo::Users::new_with_refreshed_list();
        collector.collect(&users, 1.);
        let began = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(250));
        let next = collector.collect(&users, began.elapsed().as_secs_f64());
        // Reap before asserting, so a failed check cannot leave a busy child behind.
        child.kill().unwrap();
        child.wait().unwrap();
        let busy = next.iter().find(|p| p.pid == pid).unwrap();
        assert!(busy.cpu > 0. && busy.cpu <= 130., "CPU: {}", busy.cpu);
        assert!(busy.cpu_time_ms > 0);
        collector.collect(&users, 1.);
        assert!(!collector.cache.contains_key(&pid));
    }
}
