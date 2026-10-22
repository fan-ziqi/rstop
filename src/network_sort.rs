//! Sort the displayed network values numerically; missing values stay last in either direction.
use crate::{
    connections::Connection,
    model::{Network, bytes},
};
use std::cmp::Ordering;

fn optional<T: PartialOrd>(a: Option<T>, b: Option<T>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let cmp = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
            if descending { cmp.reverse() } else { cmp }
        }
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        _ => Ordering::Equal,
    }
}
fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v >= 0.)
}
fn text(value: &str) -> Option<&str> {
    (!value.is_empty() && value != "UNKNOWN").then_some(value)
}
fn port(value: &str) -> Option<u16> {
    value.rsplit_once(':')?.1.parse().ok()
}
fn total_rate(c: &Connection) -> Option<f64> {
    finite(c.rx_per_sec)
        .zip(finite(c.tx_per_sec))
        .map(|(a, b)| a + b)
}
fn total_bytes(c: &Connection) -> Option<u64> {
    c.received.zip(c.sent).map(|(a, b)| a.saturating_add(b))
}
fn rate(value: Option<f64>) -> String {
    finite(value)
        .map(|v| format!("{}/s", bytes(v)))
        .unwrap_or_else(|| "—".into())
}
fn size(value: Option<u64>) -> String {
    value.map(|v| bytes(v as f64)).unwrap_or_else(|| "—".into())
}
fn number<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ConnectionSort {
    Traffic,
    Process,
    State,
    Remote,
    Pid,
    Protocol,
    Local,
    Download,
    Upload,
    Received,
    Sent,
    Rtt,
    ReceiveQueue,
    SendQueue,
    Retransmits,
    TotalBytes,
    LocalPort,
    RemotePort,
}
impl ConnectionSort {
    pub const ALL: [Self; 18] = [
        Self::Traffic,
        Self::Process,
        Self::State,
        Self::Remote,
        Self::Pid,
        Self::Protocol,
        Self::Local,
        Self::Download,
        Self::Upload,
        Self::Received,
        Self::Sent,
        Self::Rtt,
        Self::ReceiveQueue,
        Self::SendQueue,
        Self::Retransmits,
        Self::TotalBytes,
        Self::LocalPort,
        Self::RemotePort,
    ];
    pub fn at(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Traffic)
    }
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|v| *v == self).unwrap_or(0)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Traffic => "Combined download + upload rate",
            Self::Process => "Process name",
            Self::State => "Connection state",
            Self::Remote => "Destination address",
            Self::Pid => "Process ID (PID)",
            Self::Protocol => "Protocol",
            Self::Local => "Local address",
            Self::Download => "Download rate",
            Self::Upload => "Upload rate",
            Self::Received => "Bytes received",
            Self::Sent => "Bytes sent",
            Self::Rtt => "TCP round-trip latency (RTT)",
            Self::ReceiveQueue => "Receive queue",
            Self::SendQueue => "Send queue",
            Self::Retransmits => "TCP retransmits",
            Self::TotalBytes => "Total bytes transferred",
            Self::LocalPort => "Local port",
            Self::RemotePort => "Destination port",
        }
    }
    pub fn header(self) -> &'static str {
        match self {
            Self::Traffic => "TOTAL/s",
            Self::Process => "PROCESS",
            Self::State => "STATE",
            Self::Remote => "DESTINATION",
            Self::Pid => "PID",
            Self::Protocol => "PROTO",
            Self::Local => "LOCAL",
            Self::Download => "DOWN",
            Self::Upload => "UP",
            Self::Received => "RECEIVED",
            Self::Sent => "SENT",
            Self::Rtt => "RTT ms",
            Self::ReceiveQueue => "RECV QUEUE",
            Self::SendQueue => "SEND QUEUE",
            Self::Retransmits => "RETRANS",
            Self::TotalBytes => "TOTAL BYTES",
            Self::LocalPort => "LPORT",
            Self::RemotePort => "DPORT",
        }
    }
    pub fn descending(self, reverse: bool) -> bool {
        (!matches!(
            self,
            Self::Process
                | Self::State
                | Self::Remote
                | Self::Pid
                | Self::Protocol
                | Self::Local
                | Self::LocalPort
                | Self::RemotePort
        )) != reverse
    }
    pub fn compare(self, a: &Connection, b: &Connection, reverse: bool) -> Ordering {
        let desc = self.descending(reverse);
        match self {
            Self::Process => optional(
                a.process_names.first().map(|s| s.to_lowercase()),
                b.process_names.first().map(|s| s.to_lowercase()),
                desc,
            ),
            Self::Pid => optional(a.pids.iter().min(), b.pids.iter().min(), desc),
            Self::State => optional(text(&a.state), text(&b.state), desc),
            Self::Protocol => optional(text(&a.protocol), text(&b.protocol), desc),
            Self::Local => optional(text(&a.local), text(&b.local), desc),
            Self::Remote => optional(text(&a.remote), text(&b.remote), desc),
            Self::LocalPort => optional(port(&a.local), port(&b.local), desc),
            Self::RemotePort => optional(port(&a.remote), port(&b.remote), desc),
            Self::Traffic => optional(total_rate(a), total_rate(b), desc),
            Self::Download => optional(finite(a.rx_per_sec), finite(b.rx_per_sec), desc),
            Self::Upload => optional(finite(a.tx_per_sec), finite(b.tx_per_sec), desc),
            Self::Received => optional(a.received, b.received, desc),
            Self::Sent => optional(a.sent, b.sent, desc),
            Self::TotalBytes => optional(total_bytes(a), total_bytes(b), desc),
            Self::Rtt => optional(finite(a.rtt_ms), finite(b.rtt_ms), desc),
            Self::ReceiveQueue => optional(a.receive_queue, b.receive_queue, desc),
            Self::SendQueue => optional(a.send_queue, b.send_queue, desc),
            Self::Retransmits => optional(a.retransmits, b.retransmits, desc),
        }
        .then(a.id.cmp(&b.id))
    }
    pub fn cell(self, c: &Connection) -> String {
        match self {
            Self::Process => {
                if c.process_names.is_empty() {
                    "unknown / restricted".into()
                } else {
                    c.process_names.join(", ")
                }
            }
            Self::Pid => {
                if c.pids.is_empty() {
                    "—".into()
                } else {
                    c.pids
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                }
            }
            Self::State => c.state.clone(),
            Self::Protocol => c.protocol.clone(),
            Self::Local => c.local.clone(),
            Self::Remote => c.remote.clone(),
            Self::LocalPort => number(port(&c.local)),
            Self::RemotePort => number(port(&c.remote)),
            Self::Traffic => rate(total_rate(c)),
            Self::Download => rate(c.rx_per_sec),
            Self::Upload => rate(c.tx_per_sec),
            Self::Received => size(c.received),
            Self::Sent => size(c.sent),
            Self::TotalBytes => size(total_bytes(c)),
            Self::Rtt => finite(c.rtt_ms)
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "—".into()),
            Self::ReceiveQueue => size(c.receive_queue),
            Self::SendQueue => size(c.send_queue),
            Self::Retransmits => number(c.retransmits),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceSort {
    Name,
    State,
    Download,
    Upload,
    Received,
    Sent,
    Errors,
    Drops,
    RxPackets,
    TxPackets,
    Mtu,
    Speed,
    Address,
    RxPacketRate,
    TxPacketRate,
    RxPacketSize,
    TxPacketSize,
}
impl InterfaceSort {
    pub const ALL: [Self; 17] = [
        Self::Name,
        Self::State,
        Self::Download,
        Self::Upload,
        Self::Received,
        Self::Sent,
        Self::Errors,
        Self::Drops,
        Self::RxPackets,
        Self::TxPackets,
        Self::Mtu,
        Self::Speed,
        Self::Address,
        Self::RxPacketRate,
        Self::TxPacketRate,
        Self::RxPacketSize,
        Self::TxPacketSize,
    ];
    pub fn at(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Name)
    }
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|v| *v == self).unwrap_or(0)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Name => "Interface name",
            Self::State => "Link state",
            Self::Download => "Receive rate",
            Self::Upload => "Transmit rate",
            Self::Received => "Total received bytes",
            Self::Sent => "Total transmitted bytes",
            Self::Errors => "Errors",
            Self::Drops => "Dropped packets",
            Self::RxPackets => "Received packets",
            Self::TxPackets => "Transmitted packets",
            Self::Mtu => "MTU",
            Self::Speed => "Link speed",
            Self::Address => "IP address",
            Self::RxPacketRate => "Received packets per second",
            Self::TxPacketRate => "Transmitted packets per second",
            Self::RxPacketSize => "Average received packet size",
            Self::TxPacketSize => "Average transmitted packet size",
        }
    }
    pub fn header(self) -> &'static str {
        match self {
            Self::Name => "INTERFACE",
            Self::State => "STATE",
            Self::Download => "RECEIVE",
            Self::Upload => "TRANSMIT",
            Self::Received => "RX TOTAL",
            Self::Sent => "TX TOTAL",
            Self::Errors => "ERRORS",
            Self::Drops => "DROPS",
            Self::RxPackets => "RX PACKETS",
            Self::TxPackets => "TX PACKETS",
            Self::Mtu => "MTU",
            Self::Speed => "LINK Mb/s",
            Self::Address => "ADDRESSES",
            Self::RxPacketRate => "RX PKT/s",
            Self::TxPacketRate => "TX PKT/s",
            Self::RxPacketSize => "RX B/PKT",
            Self::TxPacketSize => "TX B/PKT",
        }
    }
    pub fn descending(self, reverse: bool) -> bool {
        (!matches!(self, Self::Name | Self::State | Self::Address)) != reverse
    }
    pub fn compare(self, a: &Network, b: &Network, reverse: bool) -> Ordering {
        let desc = self.descending(reverse);
        match self {
            Self::Name => optional(text(&a.name), text(&b.name), desc),
            Self::State => optional(text(&a.state), text(&b.state), desc),
            Self::Address => optional(text(&a.address), text(&b.address), desc),
            Self::Download => {
                optional(finite(Some(a.rx_per_sec)), finite(Some(b.rx_per_sec)), desc)
            }
            Self::Upload => optional(finite(Some(a.tx_per_sec)), finite(Some(b.tx_per_sec)), desc),
            Self::Received => optional(Some(a.rx_total), Some(b.rx_total), desc),
            Self::Sent => optional(Some(a.tx_total), Some(b.tx_total), desc),
            Self::Errors => optional(Some(a.errors), Some(b.errors), desc),
            Self::Drops => optional(a.drops, b.drops, desc),
            Self::RxPackets => optional(Some(a.rx_packets), Some(b.rx_packets), desc),
            Self::TxPackets => optional(Some(a.tx_packets), Some(b.tx_packets), desc),
            Self::Mtu => optional(Some(a.mtu), Some(b.mtu), desc),
            Self::Speed => optional(a.speed_mbps, b.speed_mbps, desc),
            Self::RxPacketRate => optional(
                finite(a.rx_packets_per_sec),
                finite(b.rx_packets_per_sec),
                desc,
            ),
            Self::TxPacketRate => optional(
                finite(a.tx_packets_per_sec),
                finite(b.tx_packets_per_sec),
                desc,
            ),
            Self::RxPacketSize => optional(
                packet_size(a.rx_per_sec, a.rx_packets_per_sec),
                packet_size(b.rx_per_sec, b.rx_packets_per_sec),
                desc,
            ),
            Self::TxPacketSize => optional(
                packet_size(a.tx_per_sec, a.tx_packets_per_sec),
                packet_size(b.tx_per_sec, b.tx_packets_per_sec),
                desc,
            ),
        }
        .then(a.name.cmp(&b.name))
    }
    pub fn cell(self, n: &Network) -> String {
        match self {
            Self::Name => n.name.clone(),
            Self::State => n.state.clone(),
            Self::Address => n.address.clone(),
            Self::Download => rate(Some(n.rx_per_sec)),
            Self::Upload => rate(Some(n.tx_per_sec)),
            Self::Received => size(Some(n.rx_total)),
            Self::Sent => size(Some(n.tx_total)),
            Self::Errors => n.errors.to_string(),
            Self::Drops => number(n.drops),
            Self::RxPackets => n.rx_packets.to_string(),
            Self::TxPackets => n.tx_packets.to_string(),
            Self::Mtu => n.mtu.to_string(),
            Self::Speed => number(n.speed_mbps),
            Self::RxPacketRate => packet_number(n.rx_packets_per_sec),
            Self::TxPacketRate => packet_number(n.tx_packets_per_sec),
            Self::RxPacketSize => packet_number(packet_size(n.rx_per_sec, n.rx_packets_per_sec)),
            Self::TxPacketSize => packet_number(packet_size(n.tx_per_sec, n.tx_packets_per_sec)),
        }
    }
}

fn packet_size(bytes: f64, packets: Option<f64>) -> Option<f64> {
    finite(packets).filter(|v| *v > 0.).map(|v| bytes / v)
}
fn packet_number(value: Option<f64>) -> String {
    finite(value)
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "—".into())
}
