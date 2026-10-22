//! The signal picker uses the platform's supported set for drawing and navigation.
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use sysinfo::Signal;

pub struct State {
    pub offset: usize,
    pub columns: usize,
    pub rows: usize,
    pub hits: Vec<(Rect, Signal)>,
    pub target_scroll: usize,
    pub targets_rect: Rect,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: 0,
            columns: 2,
            rows: 8,
            hits: vec![],
            target_scroll: 0,
            targets_rect: Rect::default(),
        }
    }
}

pub struct Choice {
    pub name: &'static str,
    pub description: &'static str,
    pub shortcut: Option<char>,
}

pub fn describe(signal: Signal) -> Choice {
    use Signal::*;
    let (name, description, shortcut) = match signal {
        Hangup => ("SIGHUP", "Hangup", None),
        Interrupt => ("SIGINT", "Interrupt", Some('i')),
        Quit => ("SIGQUIT", "Quit / core dump", None),
        Illegal => ("SIGILL", "Illegal instruction", None),
        Trap => ("SIGTRAP", "Trace / breakpoint", None),
        Abort => ("SIGABRT", "Abort / core dump", None),
        IOT => ("SIGIOT", "IOT trap", None),
        Bus => ("SIGBUS", "Bus error", None),
        FloatingPointException => ("SIGFPE", "Arithmetic exception", None),
        Kill => ("SIGKILL", "Force kill", Some('k')),
        User1 => ("SIGUSR1", "User-defined signal 1", None),
        Segv => ("SIGSEGV", "Segmentation fault", None),
        User2 => ("SIGUSR2", "User-defined signal 2", None),
        Pipe => ("SIGPIPE", "Broken pipe", None),
        Alarm => ("SIGALRM", "Alarm timer", None),
        Term => ("SIGTERM", "Terminate", Some('t')),
        Child => ("SIGCHLD", "Child status changed", None),
        Continue => ("SIGCONT", "Continue", Some('c')),
        Stop => ("SIGSTOP", "Stop", Some('s')),
        TSTP => ("SIGTSTP", "Terminal stop", None),
        TTIN => ("SIGTTIN", "Background terminal read", None),
        TTOU => ("SIGTTOU", "Background terminal write", None),
        Urgent => ("SIGURG", "Urgent socket data", None),
        XCPU => ("SIGXCPU", "CPU time limit", None),
        XFSZ => ("SIGXFSZ", "File size limit", None),
        VirtualAlarm => ("SIGVTALRM", "Virtual timer", None),
        Profiling => ("SIGPROF", "Profiling timer", None),
        Winch => ("SIGWINCH", "Terminal resized", None),
        IO => ("SIGIO", "I/O ready", None),
        Poll => ("SIGPOLL", "Pollable event", None),
        Power => ("SIGPWR", "Power failure", None),
        Sys => ("SIGSYS", "Bad system call", None),
    };
    Choice {
        name,
        description,
        shortcut,
    }
}

/// Move through the same row-major grid that the renderer displays.
pub fn navigate(current: Signal, key: KeyCode, columns: usize, rows: usize) -> Option<Signal> {
    let signals = sysinfo::SUPPORTED_SIGNALS;
    let count = signals.len();
    if count == 0 {
        return None;
    }
    let index = signals.iter().position(|s| *s == current).unwrap_or(0);
    let columns = columns.max(1);
    let next = match key {
        KeyCode::Up if index >= columns => index - columns,
        KeyCode::Up => {
            let column = index % columns;
            column + (count - 1 - column) / columns * columns
        }
        KeyCode::Down if index + columns < count => index + columns,
        KeyCode::Down => (index % columns).min(count - 1),
        KeyCode::Left | KeyCode::BackTab => (index + count - 1) % count,
        KeyCode::Right | KeyCode::Tab => (index + 1) % count,
        KeyCode::Home => 0,
        KeyCode::End => count - 1,
        KeyCode::PageUp => index.saturating_sub(columns * rows.max(1)),
        KeyCode::PageDown => (index + columns * rows.max(1)).min(count - 1),
        _ => return None,
    };
    signals.get(next).copied()
}
