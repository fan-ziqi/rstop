#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Cpu,
    Memory,
    GpuMemory,
    Io,
    Pid,
    Name,
    User,
    Gpu,
    Read,
    Write,
    Vms,
    Nice,
    Time,
    Rx,
    Tx,
    ReadTotal,
    WriteTotal,
    State,
    Field(u8),
}
impl Sort {
    pub const ALL: &'static [Self] = &[
        Self::Cpu,
        Self::Memory,
        Self::Gpu,
        Self::GpuMemory,
        Self::Read,
        Self::Write,
        Self::Io,
        Self::Time,
        Self::Pid,
        Self::Name,
        Self::User,
        Self::Nice,
        Self::Vms,
        Self::Rx,
        Self::Tx,
        Self::ReadTotal,
        Self::WriteTotal,
        Self::State,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "RAM",
            Self::GpuMemory => "VRAM",
            Self::Io => "I/O",
            Self::Pid => "PID",
            Self::Name => "NAME",
            Self::User => "USER",
            Self::Gpu => "GPU",
            Self::Read => "READ/s",
            Self::Write => "WRITE/s",
            Self::Vms => "VIRTUAL MEMORY",
            Self::Nice => "NICE",
            Self::Time => "CPU TIME",
            Self::Rx => "NETWORK RX",
            Self::Tx => "NETWORK TX",
            Self::ReadTotal => "TOTAL READ",
            Self::WriteTotal => "TOTAL WRITE",
            Self::State => "STATE",
            Self::Field(index) => crate::fields::PROCESS_COLUMNS
                .get(index as usize)
                .copied()
                .unwrap_or("FIELD"),
        }
    }
    pub fn descending(self) -> bool {
        if let Self::Field(index) = self {
            return ![
                "ppid",
                "uid",
                "tgid",
                "pgrp",
                "session",
                "tty",
                "processor",
                "start",
                "cgroup",
                "autogroup",
                "io_priority",
                "comm",
                "exe",
                "cwd",
                "security",
                "wchan",
                "capabilities",
                "nspid",
                "scheduling",
            ]
            .contains(&crate::fields::PROCESS_COLUMNS[index as usize]);
        }
        !matches!(
            self,
            Self::Pid | Self::Name | Self::User | Self::Nice | Self::State
        )
    }
    pub fn stored(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .unwrap_or_else(|| {
                if let Self::Field(index) = self {
                    Self::ALL.len() + index as usize
                } else {
                    0
                }
            })
    }
    pub fn from_stored(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_else(|| {
            Self::Field(
                index
                    .saturating_sub(Self::ALL.len())
                    .min(crate::fields::PROCESS_COLUMNS.len() - 1) as u8,
            )
        })
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|s| s.name().eq_ignore_ascii_case(name))
            .or_else(|| {
                crate::fields::PROCESS_COLUMNS
                    .iter()
                    .position(|c| c.eq_ignore_ascii_case(name))
                    .map(|i| Self::Field(i as u8))
            })
    }
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|s| *s == self).unwrap_or(0) + 1) % Self::ALL.len()]
    }
    pub fn for_column(column: &str) -> Option<Self> {
        Some(match column {
            "pid" => Self::Pid,
            "user" => Self::User,
            "cpu" => Self::Cpu,
            "memory" => Self::Memory,
            "vms" => Self::Vms,
            "vram" => Self::GpuMemory,
            "gpu" => Self::Gpu,
            "read" => Self::Read,
            "write" => Self::Write,
            "rx" => Self::Rx,
            "tx" => Self::Tx,
            "state" => Self::State,
            "nice" => Self::Nice,
            "time" => Self::Time,
            "read_total" => Self::ReadTotal,
            "write_total" => Self::WriteTotal,
            "command" => Self::Name,
            _ => {
                return crate::fields::PROCESS_COLUMNS
                    .iter()
                    .position(|c| *c == column)
                    .map(|i| Self::Field(i as u8));
            }
        })
    }
}
