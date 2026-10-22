use crate::model::Process;
use anyhow::{Context, Result};
use std::{collections::BTreeSet, time::Duration};
pub fn validate(pid: u32, start: u64) -> Result<()> {
    anyhow::ensure!(pid > 1 && pid != std::process::id(), "Protected process");
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let p = system.process(pid).context("Process exited")?;
    anyhow::ensure!(p.start_time() == start, "PID identity changed");
    Ok(())
}
pub fn cpu_list(text: &str) -> Result<Vec<usize>> {
    let mut result = BTreeSet::new();
    for part in text.split(',') {
        let part = part.trim();
        anyhow::ensure!(!part.is_empty(), "Empty CPU list");
        let (a, b) = part.split_once('-').unwrap_or((part, part));
        let a: usize = a.parse()?;
        let b: usize = b.parse()?;
        anyhow::ensure!(
            a <= b && b < 1024,
            "CPU range must be increasing and below 1024"
        );
        result.extend(a..=b);
    }
    Ok(result.into_iter().collect())
}
pub fn priority(p: &Process, value: i32) -> Result<String> {
    validate(p.pid, p.start_time)?;
    #[cfg(unix)]
    {
        anyhow::ensure!((-20..=19).contains(&value), "nice must be -20..=19");
        let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, p.pid, value) };
        anyhow::ensure!(result == 0, "{}", std::io::Error::last_os_error());
        Ok(format!("PID {} nice = {value}", p.pid))
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::*;
        let class = match value {
            v if v >= 15 => IDLE_PRIORITY_CLASS,
            v if v > 0 => BELOW_NORMAL_PRIORITY_CLASS,
            0 => NORMAL_PRIORITY_CLASS,
            v if v >= -5 => ABOVE_NORMAL_PRIORITY_CLASS,
            _ => HIGH_PRIORITY_CLASS,
        };
        let h = unsafe {
            OpenProcess(
                PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                p.pid,
            )
        };
        anyhow::ensure!(!h.is_null(), "{}", std::io::Error::last_os_error());
        let result = unsafe { SetPriorityClass(h, class) };
        unsafe { CloseHandle(h) };
        anyhow::ensure!(result != 0, "{}", std::io::Error::last_os_error());
        Ok(format!("PID {} Windows priority class = {class}", p.pid))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = value;
        anyhow::bail!("Priority control is unavailable on this OS")
    }
}

pub fn parse_io_priority(text: &str) -> Result<(i32, i32)> {
    let mut fields = text.split_whitespace();
    let class = match fields.next() {
        Some("none") => 0,
        Some("realtime") => 1,
        Some("best-effort") => 2,
        Some("idle") => 3,
        _ => anyhow::bail!("Use none, idle, realtime 0..7, or best-effort 0..7"),
    };
    let level = fields
        .next()
        .map(str::parse::<i32>)
        .transpose()?
        .unwrap_or(0);
    anyhow::ensure!(
        fields.next().is_none()
            && (0..=7).contains(&level)
            && (class == 1 || class == 2 || level == 0),
        "I/O level must be 0..7; none/idle use level 0"
    );
    Ok((class, level))
}
pub fn io_priority(p: &Process, text: &str) -> Result<String> {
    let (class, level) = parse_io_priority(text)?;
    validate(p.pid, p.start_time)?;
    #[cfg(target_os = "linux")]
    {
        let result =
            unsafe { libc::syscall(libc::SYS_ioprio_set, 1, p.pid, (class << 13) | level) };
        anyhow::ensure!(result == 0, "{}", std::io::Error::last_os_error());
        Ok(format!("PID {} I/O priority = {text}", p.pid))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (class, level);
        anyhow::bail!("I/O priority is unavailable on this OS")
    }
}
pub fn autogroup_priority(p: &Process, value: i32) -> Result<String> {
    anyhow::ensure!(
        (-20..=19).contains(&value),
        "autogroup nice must be -20..=19"
    );
    validate(p.pid, p.start_time)?;
    #[cfg(target_os = "linux")]
    {
        std::fs::write(format!("/proc/{}/autogroup", p.pid), value.to_string())?;
        Ok(format!("PID {} autogroup nice = {value}", p.pid))
    }
    #[cfg(not(target_os = "linux"))]
    {
        anyhow::bail!("Autogroup priority is unavailable on this OS")
    }
}
pub fn affinity(p: &Process, text: &str) -> Result<String> {
    let cpus = cpu_list(text)?;
    validate(p.pid, p.start_time)?;
    #[cfg(target_os = "linux")]
    {
        let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        unsafe { libc::CPU_ZERO(&mut set) };
        for cpu in cpus {
            anyhow::ensure!(
                cpu < (libc::CPU_SETSIZE as usize),
                "CPU index exceeds platform capacity"
            );
            unsafe { libc::CPU_SET(cpu, &mut set) };
        }
        let result =
            unsafe { libc::sched_setaffinity(p.pid as i32, std::mem::size_of_val(&set), &set) };
        anyhow::ensure!(result == 0, "{}", std::io::Error::last_os_error());
        Ok(format!("PID {} affinity = {text}", p.pid))
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::*;
        let mut mask = 0usize;
        for cpu in cpus {
            anyhow::ensure!(
                cpu < usize::BITS as usize,
                "CPU is outside this processor group's mask"
            );
            mask |= 1usize << cpu;
        }
        let h = unsafe {
            OpenProcess(
                PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION,
                0,
                p.pid,
            )
        };
        anyhow::ensure!(!h.is_null(), "{}", std::io::Error::last_os_error());
        let result = unsafe { SetProcessAffinityMask(h, mask) };
        unsafe { CloseHandle(h) };
        anyhow::ensure!(result != 0, "{}", std::io::Error::last_os_error());
        Ok(format!("PID {} affinity = {text}", p.pid))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = cpus;
        anyhow::bail!("This OS does not expose strict per-process CPU affinity")
    }
}
pub fn inspect(p: &Process, kind: &str) -> Result<String> {
    let pid = p.pid.to_string();
    validate(p.pid, p.start_time)?;
    #[cfg(target_os = "linux")]
    match kind {
        "locks" => {
            return Ok(std::fs::read_to_string("/proc/locks")?
                .lines()
                .filter(|line| line.split_whitespace().nth(4) == Some(pid.as_str()))
                .collect::<Vec<_>>()
                .join("\n"));
        }
        "stack" => {
            return crate::discovery::command_partial(
                "gdb",
                &[
                    "--batch",
                    "--nx",
                    "--quiet",
                    "-p",
                    &pid,
                    "-ex",
                    "set pagination off",
                    "-ex",
                    "thread apply all bt",
                    "-ex",
                    "detach",
                    "-ex",
                    "quit",
                ],
                Duration::from_secs(5),
            );
        }
        "trace" => {
            return crate::discovery::command_partial(
                "strace",
                &["-f", "-tt", "-T", "-p", &pid, "-s", "128"],
                Duration::from_secs(2),
            );
        }
        "affinity" => {
            let output = std::fs::read_to_string(format!("/proc/{pid}/status"))?;
            return Ok(output
                .lines()
                .filter(|l| {
                    l.starts_with("Cpus_allowed")
                        || l.starts_with("Mems_allowed")
                        || l.starts_with("Threads:")
                        || l.starts_with("voluntary_ctxt")
                        || l.starts_with("nonvoluntary_ctxt")
                })
                .collect::<Vec<_>>()
                .join("\n"));
        }
        _ => {}
    }
    #[cfg(target_os = "macos")]
    match kind {
        "stack" => {
            return crate::discovery::command_partial(
                "sample",
                &[&pid, "1", "1"],
                Duration::from_secs(4),
            );
        }
        "trace" => {
            return crate::discovery::command_partial(
                "dtruss",
                &["-p", &pid],
                Duration::from_secs(2),
            );
        }
        "files" | "locks" => {
            return crate::discovery::command("lsof", &["-nP", "-p", &pid], Duration::from_secs(3));
        }
        _ => {}
    }
    #[cfg(windows)]
    match kind {
        "files" => {
            return crate::discovery::command(
                "handle.exe",
                &["-nobanner", "-p", &pid],
                Duration::from_secs(3),
            );
        }
        "stack" => {
            return crate::discovery::command(
                "cdb.exe",
                &["-pv", "-p", &pid, "-c", "~* k;q"],
                Duration::from_secs(5),
            );
        }
        _ => {}
    }
    let _ = kind;
    anyhow::bail!("Requested inspection backend is not available on this platform")
}
