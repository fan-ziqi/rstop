pub mod advanced;
pub mod app;
pub mod applications;
pub mod collector;
pub mod config;
pub mod connections;
pub mod diagnostics;
pub mod discovery;
#[cfg(unix)]
mod docker;
pub mod events;
pub mod fields;
pub mod filter;
pub mod gpu;
pub mod history;
#[cfg(target_os = "linux")]
mod inet_diag;
pub mod logs;
pub mod management;
pub mod model;
pub mod network_sort;
pub mod network_tools;
pub mod platform_gpu;
pub mod plot;
pub mod process_tools;
#[cfg(target_os = "linux")]
mod procfs;
pub mod project;
pub mod recording;
pub mod ros_tools;
#[cfg(target_os = "linux")]
mod systemd;
pub mod tags;
pub mod taskstats;
pub mod ui;
pub mod windows_inspect;

use std::sync::atomic::AtomicBool;
pub static STOP: AtomicBool = AtomicBool::new(false);
