//! Optional Linux TASKSTATS delay counters. Never changes kernel settings.
#[cfg(target_os = "linux")]
mod linux {
    use anyhow::{Context, Result, ensure};
    use std::{
        collections::HashMap,
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
        time::{Duration, Instant},
    };
    pub struct Client {
        fd: OwnedFd,
        family: u16,
        sequence: u32,
    }
    impl Client {
        pub fn new() -> Result<Self> {
            let fd = unsafe {
                libc::socket(
                    libc::AF_NETLINK,
                    libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                    libc::NETLINK_GENERIC,
                )
            };
            ensure!(fd >= 0, "{}", std::io::Error::last_os_error());
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };
            let timeout = libc::timeval {
                tv_sec: 0,
                tv_usec: 50_000,
            };
            ensure!(
                unsafe {
                    libc::setsockopt(
                        fd.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_RCVTIMEO,
                        (&timeout as *const libc::timeval).cast(),
                        std::mem::size_of_val(&timeout) as u32,
                    )
                } == 0,
                "{}",
                std::io::Error::last_os_error()
            );
            let mut client = Self {
                fd,
                family: 0,
                sequence: 0,
            };
            let reply = client.request(16, 3, 2, b"TASKSTATS\0")?;
            let family = super::attributes(&reply)
                .into_iter()
                .find(|(id, _)| *id == 1)
                .context("TASKSTATS family missing")?
                .1;
            ensure!(family.len() >= 2, "Invalid TASKSTATS family");
            client.family = u16::from_ne_bytes(family[..2].try_into()?);
            Ok(client)
        }
        fn request(
            &mut self,
            family: u16,
            command: u8,
            attribute: u16,
            payload: &[u8],
        ) -> Result<Vec<u8>> {
            self.sequence = self.sequence.wrapping_add(1);
            let length = 4 + payload.len();
            let size = 20 + length.next_multiple_of(4);
            let mut msg = vec![0u8; size];
            msg[..4].copy_from_slice(&(size as u32).to_ne_bytes());
            msg[4..6].copy_from_slice(&family.to_ne_bytes());
            msg[6..8].copy_from_slice(&1u16.to_ne_bytes());
            msg[8..12].copy_from_slice(&self.sequence.to_ne_bytes());
            msg[16] = command;
            msg[17] = 1;
            msg[20..22].copy_from_slice(&(length as u16).to_ne_bytes());
            msg[22..24].copy_from_slice(&attribute.to_ne_bytes());
            msg[24..24 + payload.len()].copy_from_slice(payload);
            let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_nl>() };
            address.nl_family = libc::AF_NETLINK as u16;
            ensure!(
                unsafe {
                    libc::sendto(
                        self.fd.as_raw_fd(),
                        msg.as_ptr().cast(),
                        msg.len(),
                        0,
                        (&address as *const libc::sockaddr_nl).cast(),
                        std::mem::size_of_val(&address) as u32,
                    )
                } == msg.len() as isize,
                "{}",
                std::io::Error::last_os_error()
            );
            let mut reply = vec![0u8; 16384];
            let read = unsafe {
                libc::recv(
                    self.fd.as_raw_fd(),
                    reply.as_mut_ptr().cast(),
                    reply.len(),
                    0,
                )
            };
            ensure!(
                read >= 20,
                "TASKSTATS unavailable: {}",
                std::io::Error::last_os_error()
            );
            reply.truncate(read as usize);
            let kind = u16::from_ne_bytes(reply[4..6].try_into()?);
            if kind == 2 {
                let errno = i32::from_ne_bytes(reply[16..20].try_into()?);
                return Err(std::io::Error::from_raw_os_error(-errno))
                    .context("TASKSTATS requires kernel support/CAP_NET_ADMIN");
            }
            ensure!(
                u32::from_ne_bytes(reply[8..12].try_into()?) == self.sequence,
                "TASKSTATS sequence mismatch"
            );
            Ok(reply[20..].to_vec())
        }
        pub fn delays(&mut self, pid: u32, thread: bool) -> Result<[u64; 3]> {
            let reply = self.request(
                self.family,
                1,
                if thread { 1 } else { 2 },
                &pid.to_ne_bytes(),
            )?;
            for (kind, bytes) in super::attributes(&reply) {
                if kind == 4 || kind == 5 {
                    for (kind, stats) in super::attributes(bytes) {
                        if kind == 3 {
                            return super::parse_delays(stats)
                                .context("Truncated TASKSTATS record");
                        }
                    }
                }
            }
            anyhow::bail!("TASKSTATS record missing")
        }
    }
    #[derive(Default)]
    pub struct Collector {
        client: Option<Client>,
        failure: Option<(String, Instant)>,
        previous: HashMap<(u32, u64), ([u64; 3], Instant)>,
    }
    impl Collector {
        pub fn collect(&mut self, pid: u32, start: u64, thread: bool) -> Result<serde_json::Value> {
            if let Some((error, at)) = &self.failure
                && at.elapsed() < Duration::from_secs(30)
            {
                anyhow::bail!("{error}");
            }
            let result = (|| -> Result<serde_json::Value> {
                if std::fs::read_to_string("/proc/sys/kernel/task_delayacct")
                    .ok()
                    .is_some_and(|s| s.trim() == "0")
                {
                    anyhow::bail!("kernel.task_delayacct is disabled; counters are unavailable");
                }
                if self.client.is_none() {
                    self.client = Some(Client::new()?);
                }
                let new = self.client.as_mut().unwrap().delays(pid, thread)?;
                let now = Instant::now();
                let rates = self.previous.get(&(pid, start)).map(|(old, at)| {
                    let seconds = now.duration_since(*at).as_secs_f64();
                    new.iter()
                        .zip(old)
                        .map(|(n, o)| n.checked_sub(*o).map(|v| v as f64 / 1e9 / seconds * 100.))
                        .collect::<Vec<_>>()
                });
                self.previous.insert((pid, start), (new, now));
                Ok(
                    serde_json::json!({"cpu_delay":rates.as_ref().and_then(|r|r[0]),"io_delay":rates.as_ref().and_then(|r|r[1]),"swap_delay":rates.as_ref().and_then(|r|r[2]),"taskstats_delay_total_ns":new,"delay_scope":if thread{"thread"}else{"thread group"}}),
                )
            })();
            if let Err(error) = &result {
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.raw_os_error() == Some(libc::ESRCH))
                {
                    return result;
                }
                self.failure = Some((error.to_string(), Instant::now()));
                self.client = None;
            } else {
                self.failure = None;
            }
            result
        }
        pub fn retain(&mut self, identities: &std::collections::HashSet<(u32, u64)>) {
            self.previous.retain(|key, _| identities.contains(key));
        }
    }
}
#[cfg(target_os = "linux")]
pub use linux::Collector;
/// Netlink attributes are native-endian and padded to four bytes.
pub fn attributes(mut bytes: &[u8]) -> Vec<(u16, &[u8])> {
    let mut values = vec![];
    while bytes.len() >= 4 {
        let size = u16::from_ne_bytes([bytes[0], bytes[1]]) as usize;
        if size < 4 || size > bytes.len() {
            break;
        }
        let kind = u16::from_ne_bytes([bytes[2], bytes[3]]) & 0x3fff;
        values.push((kind, &bytes[4..size]));
        let skip = size.next_multiple_of(4);
        if skip > bytes.len() {
            break;
        }
        bytes = &bytes[skip..];
    }
    values
}
pub fn parse_delays(bytes: &[u8]) -> Option<[u64; 3]> {
    if bytes.len() < 64 || u16::from_ne_bytes(bytes[..2].try_into().ok()?) < 1 {
        return None;
    }
    Some(
        [24, 40, 56]
            .map(|offset| u64::from_ne_bytes(bytes[offset..offset + 8].try_into().unwrap())),
    )
}
