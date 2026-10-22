use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub const THEMES: &[&str] = &[
    "midnight", "nord", "ember", "black", "white", "graphite", "ocean", "forest", "sand",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub interval_ms: u64,
    pub collect_gpu: bool,
    pub theme: String,
    pub view: usize,
    pub sort_index: usize,
    pub reverse_sort: bool,
    pub tree: bool,
    pub history: usize,
    pub time_window_seconds: u64,
    pub show_loopback: bool,
    pub cpu_warning: f32,
    pub memory_warning: f64,
    pub gpu_temperature_warning: u32,
    pub show_threads: bool,
    pub show_kernel_threads: bool,
    pub ascii: bool,
    pub no_color: bool,
    pub layout: String,
    pub custom_layout: Option<LayoutNode>,
    pub columns: Vec<String>,
    pub custom_colors: std::collections::BTreeMap<String, String>,
    pub monitored_folders: Vec<PathBuf>,
    pub process_network: bool,
    pub alert_hold_seconds: u64,
    pub alert_history: usize,
    pub alert_commands: Vec<Vec<String>>,
    pub log_lines: usize,
    pub record_max_mib: u64,
    pub probes: Vec<Probe>,
    pub display: Display,
    pub check_updates: bool,
    pub ignored_update_version: Option<String>,
    #[serde(skip, default = "config_exec_enabled")]
    pub config_exec: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            collect_gpu: true,
            theme: "midnight".into(),
            view: 0,
            sort_index: 0,
            reverse_sort: false,
            tree: false,
            history: 180,
            time_window_seconds: 180,
            show_loopback: false,
            cpu_warning: 90.0,
            memory_warning: 90.0,
            gpu_temperature_warning: 85,
            show_threads: false,
            show_kernel_threads: true,
            ascii: false,
            no_color: false,
            layout: "balanced".into(),
            custom_layout: None,
            columns: vec![],
            custom_colors: Default::default(),
            monitored_folders: vec![],
            process_network: false,
            alert_hold_seconds: 3,
            alert_history: 200,
            alert_commands: vec![],
            log_lines: 500,
            record_max_mib: 64,
            probes: vec![],
            display: Display::default(),
            check_updates: true,
            ignored_update_version: None,
            config_exec: true,
        }
    }
}
fn config_exec_enabled() -> bool {
    true
}

impl Config {
    pub const SETTINGS_COUNT: usize = 32;
    pub fn cycle_theme(&mut self, forward: bool) {
        let current = THEMES
            .iter()
            .position(|name| *name == self.theme)
            .unwrap_or(0);
        let step = if forward { 1 } else { THEMES.len() - 1 };
        self.theme = THEMES[(current + step) % THEMES.len()].into();
    }

    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                {
                    PathBuf::from(
                        std::env::var_os("APPDATA")
                            .or_else(|| std::env::var_os("USERPROFILE"))
                            .unwrap_or_default(),
                    )
                }
                #[cfg(target_os = "macos")]
                {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                        .join("Library/Application Support")
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
                }
            });
        base.join("rstop/config.toml")
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        let config: Self = match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text)
                .with_context(|| format!("Invalid config: {}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error.into()),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        self.display.validate()?;
        anyhow::ensure!(
            self.time_window_seconds <= 36_000,
            "time_window_seconds must be 0..=36000 (0 = cached samples)"
        );
        anyhow::ensure!(
            (250..=10000).contains(&self.interval_ms),
            "interval_ms must be 250..=10000"
        );
        anyhow::ensure!(
            (30..=3600).contains(&self.history),
            "history must be 30..=3600 samples"
        );
        anyhow::ensure!(
            THEMES.contains(&self.theme.as_str()),
            "theme must be one of: {}",
            THEMES.join(", ")
        );
        anyhow::ensure!(self.view < 12, "view must be 0..=11");
        anyhow::ensure!(
            self.sort_index < crate::app::Sort::ALL.len() + crate::fields::PROCESS_COLUMNS.len(),
            "Invalid sort_index"
        );
        anyhow::ensure!(
            ["balanced", "compute", "storage", "minimal"].contains(&self.layout.as_str()),
            "invalid layout"
        );
        anyhow::ensure!(
            (1..=10000).contains(&self.alert_history),
            "alert_history must be 1..=10000"
        );
        anyhow::ensure!(
            (1..=100000).contains(&self.log_lines),
            "log_lines must be 1..=100000"
        );
        anyhow::ensure!(
            (1..=4096).contains(&self.record_max_mib),
            "record_max_mib must be 1..=4096"
        );
        if let Some(layout) = &self.custom_layout {
            layout.validate(0)?;
        }
        for column in &self.columns {
            anyhow::ensure!(
                crate::fields::PROCESS_COLUMNS.contains(&column.as_str()),
                "Unknown column: {column}"
            );
        }
        anyhow::ensure!(
            self.alert_hold_seconds <= 3600,
            "alert_hold_seconds must be 0..=3600"
        );
        anyhow::ensure!(
            self.probes.len() <= 64 && self.monitored_folders.len() <= 64,
            "At most 64 probes/folders"
        );
        anyhow::ensure!(
            self.columns.len() <= crate::fields::PROCESS_COLUMNS.len(),
            "Too many process columns"
        );
        anyhow::ensure!(self.alert_commands.len() <= 8, "At most 8 alert commands");
        for args in &self.alert_commands {
            anyhow::ensure!(
                !args.is_empty() && !args[0].is_empty(),
                "Alert command needs a program"
            );
        }
        for (key, color) in &self.custom_colors {
            anyhow::ensure!(
                ["background", "panel", "text", "accent", "border"].contains(&key.as_str()),
                "Unknown color: {key}"
            );
            anyhow::ensure!(
                color.len() == 7
                    && color.starts_with('#')
                    && u32::from_str_radix(&color[1..], 16).is_ok(),
                "Color must be #RRGGBB"
            );
        }
        for probe in &self.probes {
            anyhow::ensure!(
                !probe.name.is_empty()
                    && !probe.command.is_empty()
                    && !probe.command[0].is_empty()
                    && probe.timeout_seconds > 0
                    && probe.timeout_seconds <= 30,
                "Invalid local probe"
            );
        }
        anyhow::ensure!(
            (0.0..=100.0).contains(&self.cpu_warning),
            "cpu_warning must be 0..=100"
        );
        anyhow::ensure!(
            (0.0..=100.0).contains(&self.memory_warning),
            "memory_warning must be 0..=100"
        );
        Ok(())
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temporary, toml::to_string_pretty(self)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Display {
    pub cpu_graph: String,
    pub cpu_graph_lower: String,
    pub temperature_unit: String,
    pub cpu_sensor: String,
    pub network_interface: String,
    pub network_bits: bool,
    pub network_binary: bool,
    pub network_logarithmic: bool,
    pub network_start_zero: bool,
    pub network_sync_scale: bool,
    /// Fixed graph maximum in bytes/s; zero selects automatic scaling.
    pub network_scale: u64,
    pub network_swap: bool,
    pub memory_graphs: bool,
    pub show_swap: bool,
    pub show_battery: bool,
    pub process_basename: bool,
    pub group_processes: bool,
    pub overview_panel: String,
}
impl Default for Display {
    fn default() -> Self {
        Self {
            cpu_graph: "total".into(),
            cpu_graph_lower: "total".into(),
            temperature_unit: "celsius".into(),
            cpu_sensor: String::new(),
            network_interface: String::new(),
            network_bits: false,
            network_binary: true,
            network_logarithmic: false,
            network_start_zero: false,
            network_sync_scale: false,
            network_scale: 0,
            network_swap: false,
            memory_graphs: true,
            show_swap: true,
            show_battery: true,
            process_basename: false,
            group_processes: false,
            overview_panel: "all".into(),
        }
    }
}
impl Display {
    pub const CPU_FIELDS: &[&str] = &[
        "total", "user", "system", "nice", "iowait", "irq", "softirq", "steal", "idle",
    ];
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            ["all", "cpu", "memory", "gpu", "storage", "network"]
                .contains(&self.overview_panel.as_str()),
            "Invalid overview panel"
        );
        anyhow::ensure!(
            Self::CPU_FIELDS.contains(&self.cpu_graph.as_str()),
            "Invalid CPU graph metric"
        );
        anyhow::ensure!(
            self.cpu_graph_lower == "none"
                || Self::CPU_FIELDS.contains(&self.cpu_graph_lower.as_str()),
            "Invalid lower CPU graph metric"
        );
        anyhow::ensure!(
            ["celsius", "fahrenheit", "kelvin"].contains(&self.temperature_unit.as_str()),
            "Invalid temperature unit"
        );
        anyhow::ensure!(
            !self.network_interface.chars().any(char::is_control),
            "Invalid network interface"
        );
        Ok(())
    }
    pub fn temperature(&self, celsius: f64, digits: usize) -> String {
        let (value, unit) = (self.temperature_value(celsius), self.temperature_symbol());
        format!("{value:.digits$}{unit}")
    }
    pub fn temperature_value(&self, celsius: f64) -> f64 {
        match self.temperature_unit.as_str() {
            "fahrenheit" => celsius * 1.8 + 32.,
            "kelvin" => celsius + 273.15,
            _ => celsius,
        }
    }
    pub fn temperature_symbol(&self) -> &'static str {
        match self.temperature_unit.as_str() {
            "fahrenheit" => "°F",
            "kelvin" => " K",
            _ => "°C",
        }
    }
    pub fn rate(&self, bytes: f64) -> String {
        let mut value = bytes.max(0.) * if self.network_bits { 8. } else { 1. };
        let units = match (self.network_bits, self.network_binary) {
            (false, true) => ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"],
            (false, false) => ["B/s", "kB/s", "MB/s", "GB/s", "TB/s"],
            (true, true) => ["bit/s", "Kibit/s", "Mibit/s", "Gibit/s", "Tibit/s"],
            (true, false) => ["bit/s", "Kbit/s", "Mbit/s", "Gbit/s", "Tbit/s"],
        };
        let base = if self.network_binary { 1024. } else { 1000. };
        let mut index = 0;
        while value >= base && index < units.len() - 1 {
            value /= base;
            index += 1;
        }
        if index == 0 {
            format!("{value:.0} {}", units[index])
        } else {
            format!("{value:.1} {}", units[index])
        }
    }
    pub fn settings(&self) -> Vec<(&'static str, String)> {
        vec![
            ("CPU graph", self.cpu_graph.clone()),
            ("CPU lower graph", self.cpu_graph_lower.clone()),
            ("Temperature unit", self.temperature_unit.clone()),
            (
                "CPU sensor",
                if self.cpu_sensor.is_empty() {
                    "Auto".into()
                } else {
                    self.cpu_sensor.clone()
                },
            ),
            (
                "Network interface",
                if self.network_interface.is_empty() {
                    "All visible".into()
                } else {
                    self.network_interface.clone()
                },
            ),
            ("Network bits/s", self.network_bits.to_string()),
            ("Network binary units", self.network_binary.to_string()),
            ("Network logarithmic", self.network_logarithmic.to_string()),
            (
                "Network totals since start",
                self.network_start_zero.to_string(),
            ),
            ("Network shared scale", self.network_sync_scale.to_string()),
            (
                "Network graph maximum",
                if self.network_scale == 0 {
                    "Auto".into()
                } else {
                    self.rate(self.network_scale as f64)
                },
            ),
            ("Upload above download", self.network_swap.to_string()),
            ("Memory graphs", self.memory_graphs.to_string()),
            ("Show swap", self.show_swap.to_string()),
            ("Show battery", self.show_battery.to_string()),
            ("Process basename only", self.process_basename.to_string()),
            (
                "Group processes by program",
                self.group_processes.to_string(),
            ),
            ("Expanded overview panel", self.overview_panel.clone()),
        ]
    }
    pub fn adjust(&mut self, index: usize, forward: bool, networks: &[String], sensors: &[String]) {
        fn cycle(value: &mut String, choices: &[&str], forward: bool) {
            let index = choices.iter().position(|v| *v == value).unwrap_or(0);
            *value = choices[(index + if forward { 1 } else { choices.len() - 1 }) % choices.len()]
                .into();
        }
        match index {
            0 => cycle(&mut self.cpu_graph, Self::CPU_FIELDS, forward),
            1 => {
                let mut fields = vec!["none"];
                fields.extend(Self::CPU_FIELDS);
                cycle(&mut self.cpu_graph_lower, &fields, forward);
            }
            2 => cycle(
                &mut self.temperature_unit,
                &["celsius", "fahrenheit", "kelvin"],
                forward,
            ),
            3 | 4 => {
                let mut names = vec![""];
                names.extend(
                    if index == 3 { sensors } else { networks }
                        .iter()
                        .map(String::as_str),
                );
                cycle(
                    if index == 3 {
                        &mut self.cpu_sensor
                    } else {
                        &mut self.network_interface
                    },
                    &names,
                    forward,
                );
            }
            5 => self.network_bits = !self.network_bits,
            6 => self.network_binary = !self.network_binary,
            7 => self.network_logarithmic = !self.network_logarithmic,
            8 => self.network_start_zero = !self.network_start_zero,
            9 => self.network_sync_scale = !self.network_sync_scale,
            10 => {
                let values = [
                    0,
                    125_000,
                    1_250_000,
                    12_500_000,
                    125_000_000,
                    1_250_000_000,
                    12_500_000_000,
                ];
                let index = values
                    .iter()
                    .position(|v| *v == self.network_scale)
                    .unwrap_or(0);
                self.network_scale =
                    values[(index + if forward { 1 } else { values.len() - 1 }) % values.len()];
            }
            11 => self.network_swap = !self.network_swap,
            12 => self.memory_graphs = !self.memory_graphs,
            13 => self.show_swap = !self.show_swap,
            14 => self.show_battery = !self.show_battery,
            15 => self.process_basename = !self.process_basename,
            16 => self.group_processes = !self.group_processes,
            17 => cycle(
                &mut self.overview_panel,
                &["all", "cpu", "memory", "gpu", "storage", "network"],
                forward,
            ),
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub name: String,
    pub command: Vec<String>,
    #[serde(default = "probe_timeout")]
    pub timeout_seconds: u64,
}
fn probe_timeout() -> u64 {
    3
}

/// A nested row/column layout; leaf panels missing on the host are omitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LayoutNode {
    Panel {
        panel: String,
    },
    Split {
        direction: String,
        children: Vec<LayoutNode>,
        #[serde(default)]
        weights: Vec<u16>,
    },
}
impl LayoutNode {
    pub fn contains_panel(&self, name: &str) -> bool {
        match self {
            Self::Panel { panel } => panel == name,
            Self::Split { children, .. } => children.iter().any(|child| child.contains_panel(name)),
        }
    }

    fn validate(&self, depth: usize) -> Result<()> {
        anyhow::ensure!(depth < 6, "Layout nesting exceeds 6 levels");
        match self {
            Self::Panel { panel } => anyhow::ensure!(
                ["cpu", "memory", "gpu", "storage", "network", "processes"]
                    .contains(&panel.as_str()),
                "Unknown panel: {panel}"
            ),
            Self::Split {
                direction,
                children,
                weights,
            } => {
                anyhow::ensure!(
                    ["horizontal", "vertical"].contains(&direction.as_str())
                        && (1..=8).contains(&children.len()),
                    "Invalid layout split"
                );
                anyhow::ensure!(
                    weights.is_empty()
                        || (weights.len() == children.len() && weights.iter().all(|w| *w > 0)),
                    "Layout weights must match children and be positive"
                );
                for child in children {
                    child.validate(depth + 1)?;
                }
            }
        }
        Ok(())
    }
}
