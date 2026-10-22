use anyhow::{Context, Result};
use rstop::app::SignalTarget;
#[cfg(target_os = "linux")]
use std::io;
pub(super) fn send_signal(target: &SignalTarget) -> Result<()> {
    anyhow::ensure!(
        target.pid > 1 && target.pid != std::process::id(),
        "PID 0, PID 1 and rstop itself are protected"
    );
    #[cfg(target_os = "linux")]
    let pidfd = {
        use std::os::fd::{FromRawFd, OwnedFd};
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, target.pid, 0) };
        anyhow::ensure!(
            fd >= 0,
            "Cannot acquire process handle: {}",
            io::Error::last_os_error()
        );
        unsafe { OwnedFd::from_raw_fd(fd as i32) }
    };
    let pid = sysinfo::Pid::from_u32(target.pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let process = system.process(pid).context("Process already exited")?;
    anyhow::ensure!(
        process.start_time() == target.start_time,
        "PID was reused; select the new process again"
    );
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let signal = linux_signal(target.signal).context("Signal is unsupported on Linux")?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                pidfd.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        anyhow::ensure!(result == 0, "{}", io::Error::last_os_error());
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    match process.kill_with(target.signal) {
        Some(true) => Ok(()),
        Some(false) => anyhow::bail!("Permission denied or process exited"),
        None => anyhow::bail!("Signal unsupported on this platform"),
    }
}

#[cfg(target_os = "linux")]
pub(super) fn linux_signal(signal: sysinfo::Signal) -> Option<i32> {
    use sysinfo::Signal::*;
    Some(match signal {
        Hangup => libc::SIGHUP,
        Interrupt => libc::SIGINT,
        Quit => libc::SIGQUIT,
        Illegal => libc::SIGILL,
        Trap => libc::SIGTRAP,
        Abort | IOT => libc::SIGABRT,
        Bus => libc::SIGBUS,
        FloatingPointException => libc::SIGFPE,
        Kill => libc::SIGKILL,
        User1 => libc::SIGUSR1,
        Segv => libc::SIGSEGV,
        User2 => libc::SIGUSR2,
        Pipe => libc::SIGPIPE,
        Alarm => libc::SIGALRM,
        Term => libc::SIGTERM,
        Child => libc::SIGCHLD,
        Continue => libc::SIGCONT,
        Stop => libc::SIGSTOP,
        TSTP => libc::SIGTSTP,
        TTIN => libc::SIGTTIN,
        TTOU => libc::SIGTTOU,
        Urgent => libc::SIGURG,
        XCPU => libc::SIGXCPU,
        XFSZ => libc::SIGXFSZ,
        VirtualAlarm => libc::SIGVTALRM,
        Profiling => libc::SIGPROF,
        Winch => libc::SIGWINCH,
        IO | Poll => libc::SIGIO,
        Power => libc::SIGPWR,
        Sys => libc::SIGSYS,
    })
}
