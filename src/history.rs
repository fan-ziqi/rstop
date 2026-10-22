//! Bounded telemetry history. No terminal or operating-system access.
use crate::model::*;
use std::collections::{HashMap, HashSet, VecDeque};
#[derive(Default)]
pub struct ProcessHistory {
    pub cpu: VecDeque<f64>,
    pub ram: VecDeque<f64>,
    pub gpu: VecDeque<f64>,
    pub vram: VecDeque<f64>,
}
impl ProcessHistory {
    pub fn push(&mut self, p: &Process, gpu: (Option<u64>, Option<u32>), limit: usize) {
        for (history, value) in [
            (&mut self.cpu, p.cpu as f64),
            (&mut self.ram, p.memory as f64),
            (&mut self.gpu, gpu.1.map_or(f64::NAN, |v| v as f64)),
            (&mut self.vram, gpu.0.map_or(f64::NAN, |v| v as f64)),
        ] {
            history.push_back(value);
            while history.len() > limit {
                history.pop_front();
            }
        }
    }
}
#[derive(Default)]
pub struct GpuHistory {
    pub utilization: VecDeque<f64>,
    pub vram: VecDeque<f64>,
    pub memory_busy: VecDeque<f64>,
    pub encoder: VecDeque<f64>,
    pub decoder: VecDeque<f64>,
    pub power: VecDeque<f64>,
    pub temperature: VecDeque<f64>,
    pub fan: VecDeque<f64>,
    pub clock: VecDeque<f64>,
    pub memory_clock: VecDeque<f64>,
    pub pcie_rx: VecDeque<f64>,
    pub pcie_tx: VecDeque<f64>,
}
#[derive(Default)]
pub struct NetworkPeak {
    pub rx: f64,
    pub tx: f64,
}
#[derive(Default)]
pub struct NetworkHistory {
    pub rx: VecDeque<f64>,
    pub tx: VecDeque<f64>,
    pub rx_peak: f64,
    pub tx_peak: f64,
    pub rx_session: u64,
    pub tx_session: u64,
    previous: Option<(u64, u64)>,
}
impl NetworkHistory {
    fn push(&mut self, rx: f64, tx: f64, received: u64, sent: u64, limit: usize) {
        self.rx.push_back(rx);
        self.tx.push_back(tx);
        while self.rx.len() > limit {
            self.rx.pop_front();
            self.tx.pop_front();
        }
        self.rx_peak = self.rx_peak.max(rx);
        self.tx_peak = self.tx_peak.max(tx);
        self.rx_session = self.rx_session.saturating_add(received);
        self.tx_session = self.tx_session.saturating_add(sent);
    }
}
#[derive(Default)]
pub struct History {
    pub sequence: u64,
    pub timestamps: VecDeque<f64>,
    pub periods: VecDeque<f64>,
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
    pub rx: VecDeque<f64>,
    pub tx: VecDeque<f64>,
    /// Session peaks for non-loopback traffic and all traffic, respectively.
    pub network_peaks: [NetworkPeak; 2],
    pub networks: HashMap<String, NetworkHistory>,
    pub network_totals: [NetworkHistory; 2],
    pub disk_read: VecDeque<f64>,
    pub disk_write: VecDeque<f64>,
    pub gpu: HashMap<String, VecDeque<f64>>,
    pub gpu_details: HashMap<String, GpuHistory>,
    pub series: std::collections::BTreeMap<String, VecDeque<f64>>,
}

impl History {
    pub fn network(&self, config: &crate::config::Config) -> Option<&NetworkHistory> {
        if config.display.network_interface.is_empty() {
            Some(&self.network_totals[usize::from(config.show_loopback)])
        } else {
            self.networks.get(&config.display.network_interface)
        }
    }
    pub fn window(&self, samples: usize) -> crate::plot::Window<'_> {
        crate::plot::Window {
            samples,
            end: self.sequence,
            time: None,
        }
    }
    pub fn time_window(&self, seconds: u64) -> crate::plot::Window<'_> {
        crate::plot::Window {
            samples: self.timestamps.len(),
            end: self.sequence,
            time: Some(crate::plot::TimeScale {
                timestamps: &self.timestamps,
                periods: &self.periods,
                seconds,
            }),
        }
    }
    pub fn ingest(&mut self, snapshot: &Snapshot, config: &crate::config::Config) {
        let limit = config.history;
        self.sequence = self.sequence.saturating_add(1);
        let period = config.interval_ms as f64 / 1000.;
        let previous = self.timestamps.back().copied();
        let time = if snapshot.elapsed.is_finite() && previous.is_none_or(|p| snapshot.elapsed > p)
        {
            snapshot.elapsed.max(0.)
        } else {
            previous.unwrap_or(0.) + period
        };
        self.timestamps.push_back(time);
        self.periods.push_back(period);
        while self.timestamps.len() > limit {
            self.timestamps.pop_front();
            self.periods.pop_front();
        }
        let push = |history: &mut VecDeque<f64>, value: f64| {
            history.push_back(value);
            while history.len() > limit {
                history.pop_front();
            }
        };
        push(&mut self.cpu, snapshot.cpu_percent as f64);
        push(
            &mut self.memory,
            percent(snapshot.memory.used, snapshot.memory.total),
        );
        let networks: Vec<_> = snapshot
            .networks
            .iter()
            .filter(|n| config.show_loopback || !is_loopback(&n.name))
            .collect();
        push(&mut self.rx, networks.iter().map(|n| n.rx_per_sec).sum());
        push(&mut self.tx, networks.iter().map(|n| n.tx_per_sec).sum());
        for (include_loopback, peak) in self.network_peaks.iter_mut().enumerate() {
            let mut rx = 0.;
            let mut tx = 0.;
            for network in &snapshot.networks {
                if include_loopback == 1 || !is_loopback(&network.name) {
                    rx += network.rx_per_sec;
                    tx += network.tx_per_sec;
                }
            }
            peak.rx = peak.rx.max(rx);
            peak.tx = peak.tx.max(tx);
        }
        let mut aggregate = [(0., 0., 0u64, 0u64); 2];
        self.networks
            .retain(|name, _| snapshot.networks.iter().any(|n| &n.name == name));
        for network in &snapshot.networks {
            let history = self.networks.entry(network.name.clone()).or_default();
            // A reset starts a new counter epoch; its first observation is a baseline.
            let delta = history
                .previous
                .map(|(rx, tx)| {
                    (
                        network.rx_total.saturating_sub(rx),
                        network.tx_total.saturating_sub(tx),
                    )
                })
                .unwrap_or((0, 0));
            history.previous = Some((network.rx_total, network.tx_total));
            history.push(
                network.rx_per_sec,
                network.tx_per_sec,
                delta.0,
                delta.1,
                limit,
            );
            for (include_loopback, (rx, tx, received, sent)) in aggregate.iter_mut().enumerate() {
                if include_loopback == 1 || !is_loopback(&network.name) {
                    *rx += network.rx_per_sec;
                    *tx += network.tx_per_sec;
                    *received = received.saturating_add(delta.0);
                    *sent = sent.saturating_add(delta.1);
                }
            }
        }
        for (history, (rx, tx, received, sent)) in self.network_totals.iter_mut().zip(aggregate) {
            history.push(rx, tx, received, sent, limit);
        }
        push(
            &mut self.disk_read,
            snapshot
                .disk_io
                .iter()
                .filter(|d| d.aggregate)
                .map(|d| d.read_per_sec)
                .sum(),
        );
        push(
            &mut self.disk_write,
            snapshot
                .disk_io
                .iter()
                .filter(|d| d.aggregate)
                .map(|d| d.write_per_sec)
                .sum(),
        );
        for gpu in &snapshot.gpus {
            push(
                self.gpu.entry(gpu_key(gpu)).or_default(),
                gpu.utilization.map(|v| v as f64).unwrap_or(f64::NAN),
            );
        }
        let present: HashSet<_> = snapshot.gpus.iter().map(gpu_key).collect();
        self.gpu.retain(|key, _| present.contains(key));
        let present: HashSet<_> = snapshot.gpus.iter().map(|g| g.uuid.clone()).collect();
        self.gpu_details.retain(|key, _| present.contains(key));
        for gpu in &snapshot.gpus {
            let h = self.gpu_details.entry(gpu.uuid.clone()).or_default();
            push(
                &mut h.utilization,
                gpu.utilization.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.vram,
                gpu.memory_used
                    .zip(gpu.memory_total)
                    .filter(|(_, total)| *total > 0)
                    .map(|(used, total)| percent(used, total))
                    .unwrap_or(f64::NAN),
            );
            push(
                &mut h.memory_busy,
                gpu.memory_utilization.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.encoder,
                gpu.encoder.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.decoder,
                gpu.decoder.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.power,
                gpu.power_watts.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.temperature,
                gpu.temperature.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(&mut h.fan, gpu.fan.map(|v| v as f64).unwrap_or(f64::NAN));
            push(
                &mut h.clock,
                gpu.clock_mhz.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.memory_clock,
                gpu.memory_clock_mhz.map(|v| v as f64).unwrap_or(f64::NAN),
            );
            push(
                &mut h.pcie_rx,
                gpu.pcie_rx_kib
                    .map(|v| v as f64 * 1024.)
                    .unwrap_or(f64::NAN),
            );
            push(
                &mut h.pcie_tx,
                gpu.pcie_tx_kib
                    .map(|v| v as f64 * 1024.)
                    .unwrap_or(f64::NAN),
            );
        }
        let mut values = std::collections::BTreeMap::new();
        values.insert("CPU total (%)".into(), snapshot.cpu_percent as f64);
        for (name, value) in &snapshot.cpu_breakdown {
            values.insert(format!("CPU {name} (%)"), *value);
        }
        values.insert(
            "RAM (%)".into(),
            percent(snapshot.memory.used, snapshot.memory.total),
        );
        values.insert(
            "Swap (%)".into(),
            percent(snapshot.memory.swap_used, snapshot.memory.swap_total),
        );
        for (name, value) in [
            ("RAM available (%)", Some(snapshot.memory.available)),
            ("RAM cached (%)", snapshot.memory.cached),
            ("RAM free (%)", snapshot.memory.free),
        ] {
            values.insert(
                name.into(),
                value
                    .filter(|_| snapshot.memory.total > 0)
                    .map(|value| percent(value, snapshot.memory.total))
                    .unwrap_or(f64::NAN),
            );
        }
        for (index, value) in snapshot.cores.iter().enumerate() {
            values.insert(format!("CPU core {index:03} (%)"), *value as f64);
        }
        for sensor in &snapshot.sensors {
            values.insert(
                format!("Temperature {} (°C)", sensor.name),
                sensor.temperature as f64,
            );
        }
        for gpu in &snapshot.gpus {
            for (key, value) in [
                ("GPU", gpu.utilization),
                ("Encoder", gpu.encoder),
                ("Decoder", gpu.decoder),
                ("Memory", gpu.memory_utilization),
            ] {
                if let Some(value) = value {
                    values.insert(format!("{} {key} (%)", gpu.name), value as f64);
                }
            }
        }
        for network in &snapshot.networks {
            values.insert(
                format!("Network {} RX (B/s)", network.name),
                network.rx_per_sec,
            );
            values.insert(
                format!("Network {} TX (B/s)", network.name),
                network.tx_per_sec,
            );
        }
        for disk in &snapshot.disk_io {
            values.insert(format!("Disk {} read (B/s)", disk.name), disk.read_per_sec);
            values.insert(
                format!("Disk {} write (B/s)", disk.name),
                disk.write_per_sec,
            );
            if let Some(value) = disk.await_ms {
                values.insert(format!("Disk {} latency (ms)", disk.name), value);
            }
        }
        for container in &snapshot.inventory.containers {
            let key = container_key(container);
            for (metric, value) in [
                ("network RX (B/s)", container.network_rx_per_sec),
                ("network TX (B/s)", container.network_tx_per_sec),
                ("disk read (B/s)", container.block_read_per_sec),
                ("disk write (B/s)", container.block_write_per_sec),
            ] {
                values.insert(format!("{key} {metric}"), value.unwrap_or(f64::NAN));
            }
            values.insert(format!("{key} CPU (%)"), container.cpu.unwrap_or(f64::NAN));
            values.insert(
                format!("{key} memory (B)"),
                container.memory.map(|v| v as f64).unwrap_or(f64::NAN),
            );
        }
        for service in &snapshot.inventory.services {
            if let Some(memory) = service.memory {
                values.insert(
                    format!("Service {}/{} memory (B)", service.scope, service.name),
                    memory as f64,
                );
            }
            if let Some(cpu) = service.cpu_percent {
                values.insert(
                    format!("Service {}/{} main CPU (%)", service.scope, service.name),
                    cpu as f64,
                );
            }
        }
        self.series.retain(|key, _| values.contains_key(key));
        for (key, value) in values {
            push(self.series.entry(key).or_default(), value);
        }
    }
}
pub fn gpu_key(gpu: &Gpu) -> String {
    if gpu.uuid.is_empty() {
        format!("{}:{}:{}", gpu.vendor, gpu.id, gpu.name)
    } else {
        gpu.uuid.clone()
    }
}

pub fn container_key(container: &Container) -> String {
    format!(
        "Container {}/{} ({})",
        container.runtime, container.id, container.name
    )
}
