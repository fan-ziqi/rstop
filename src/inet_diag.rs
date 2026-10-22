//! Linux SOCK_DIAG UAPI: one dump supplies endpoints, queues and TCP_INFO.
//! Layouts are defined in linux/inet_diag.h and linux/tcp.h. Older TCP_INFO tails stay unknown.
use crate::connections::Connection;
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::{Duration, Instant},
};

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(data.get(at..at + 4)?.try_into().ok()?))
}
fn u64_at(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_ne_bytes(data.get(at..at + 8)?.try_into().ok()?))
}
fn endpoint(data: &[u8], address: usize, port: usize) -> Option<String> {
    let port = u16::from_be_bytes(data.get(port..port + 2)?.try_into().ok()?);
    Some(if data[0] == libc::AF_INET as u8 {
        format!(
            "{}:{port}",
            Ipv4Addr::from(<[u8; 4]>::try_from(data.get(address..address + 4)?).ok()?)
        )
    } else {
        format!(
            "[{}]:{port}",
            Ipv6Addr::from(<[u8; 16]>::try_from(data.get(address..address + 16)?).ok()?)
        )
    })
}
fn parse(data: &[u8], protocol: &str) -> Option<Connection> {
    if data.len() < 72 || ![libc::AF_INET as u8, libc::AF_INET6 as u8].contains(&data[0]) {
        return None;
    }
    let local = endpoint(data, 8, 4)?;
    let remote = endpoint(data, 24, 6)?;
    let inode = u32_at(data, 68)? as u64;
    let cookie = u64_at(data, 44)?;
    let state = match data[1] {
        1 => "ESTABLISHED",
        2 => "SYN_SENT",
        3 => "SYN_RECV",
        4 => "FIN_WAIT1",
        5 => "FIN_WAIT2",
        6 => "TIME_WAIT",
        7 if protocol.starts_with("udp") => "BOUND",
        7 => "CLOSED",
        8 => "CLOSE_WAIT",
        9 => "LAST_ACK",
        10 => "LISTEN",
        11 => "CLOSING",
        _ => "UNKNOWN",
    };
    let mut row = Connection {
        id: format!("{protocol}:{inode}:{local}:{remote}:{cookie}"),
        protocol: protocol.into(),
        local,
        remote,
        state: state.into(),
        inode,
        receive_queue: u32_at(data, 56).map(u64::from),
        send_queue: u32_at(data, 60).map(u64::from),
        ..Default::default()
    };
    let mut tail = &data[72..];
    while tail.len() >= 4 {
        let size = u16::from_ne_bytes(tail[..2].try_into().ok()?) as usize;
        let kind = u16::from_ne_bytes(tail[2..4].try_into().ok()?) & 0x3fff;
        if size < 4 || size > tail.len() {
            return None;
        }
        if kind == 2 && protocol.starts_with("tcp") {
            let info = &tail[4..size];
            row.rtt_ms = u32_at(info, 68).map(|v| v as f64 / 1000.);
            row.retransmits = u32_at(info, 100).map(u64::from);
            row.received = u64_at(info, 128);
            row.sent = u64_at(info, 200);
        }
        let aligned = (size + 3) & !3;
        if aligned > tail.len() {
            break;
        }
        tail = &tail[aligned..];
    }
    Some(row)
}

pub(crate) fn collect() -> io::Result<Vec<Connection>> {
    let raw = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_RAW | libc::SOCK_CLOEXEC,
            libc::NETLINK_SOCK_DIAG,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let socket = unsafe { OwnedFd::from_raw_fd(raw) };
    let timeout = libc::timeval {
        tv_sec: 0,
        tv_usec: 250_000,
    };
    let result = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &timeout as *const _ as *const _,
            std::mem::size_of_val(&timeout) as _,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut address: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    address.nl_family = libc::AF_NETLINK as _;
    if unsafe {
        libc::bind(
            socket.as_raw_fd(),
            &address as *const _ as *const _,
            std::mem::size_of_val(&address) as _,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let began = Instant::now();
    let mut rows = Vec::new();
    let mut buffer = vec![0u8; 65536];
    for (sequence, (family, protocol, name)) in [
        (libc::AF_INET, libc::IPPROTO_TCP, "tcp"),
        (libc::AF_INET6, libc::IPPROTO_TCP, "tcp6"),
        (libc::AF_INET, libc::IPPROTO_UDP, "udp"),
        (libc::AF_INET6, libc::IPPROTO_UDP, "udp6"),
    ]
    .into_iter()
    .enumerate()
    {
        let sequence = sequence as u32 + 1;
        let mut request = [0u8; 72];
        request[..4].copy_from_slice(&72u32.to_ne_bytes());
        request[4..6].copy_from_slice(&20u16.to_ne_bytes());
        request[6..8].copy_from_slice(&0x301u16.to_ne_bytes());
        request[8..12].copy_from_slice(&sequence.to_ne_bytes());
        request[16] = family as u8;
        request[17] = protocol as u8;
        request[18] = 2;
        request[20..24].fill(0xff);
        request[64..72].fill(0xff);
        if unsafe {
            libc::sendto(
                socket.as_raw_fd(),
                request.as_ptr() as _,
                request.len(),
                0,
                &address as *const _ as _,
                std::mem::size_of_val(&address) as _,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        'response: loop {
            if began.elapsed() > Duration::from_secs(1) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "socket dump deadline",
                ));
            }
            let size = unsafe {
                libc::recv(
                    socket.as_raw_fd(),
                    buffer.as_mut_ptr() as _,
                    buffer.len(),
                    libc::MSG_TRUNC,
                )
            };
            if size < 0 {
                return Err(io::Error::last_os_error());
            }
            if size == 0 || size as usize > buffer.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "truncated socket dump",
                ));
            }
            let mut messages = &buffer[..size as usize];
            while messages.len() >= 16 {
                let length = u32_at(messages, 0).unwrap() as usize;
                if length < 16 || length > messages.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid netlink message",
                    ));
                }
                if u32_at(messages, 8) != Some(sequence) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "socket dump sequence changed",
                    ));
                }
                let kind = u16::from_ne_bytes(messages[4..6].try_into().unwrap());
                let flags = u16::from_ne_bytes(messages[6..8].try_into().unwrap());
                if flags & 0x10 != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "socket dump interrupted",
                    ));
                }
                let data = &messages[16..length];
                match kind {
                    3 => break 'response,
                    2 => {
                        let error = u32_at(data, 0).ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "missing netlink error")
                        })? as i32;
                        if error != 0 {
                            return Err(io::Error::from_raw_os_error(-error));
                        }
                    }
                    20 => {
                        if let Some(row) = parse(data, name) {
                            rows.push(row);
                        } else {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid socket record",
                            ));
                        }
                    }
                    _ => {}
                }
                if rows.len() > 20_000 {
                    return Ok(rows);
                }
                let aligned = (length + 3) & !3;
                if aligned > messages.len() {
                    break;
                }
                messages = &messages[aligned..];
            }
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_fields_keep_units_cookie_and_short_tcp_info_unknowns() {
        let mut data = vec![0u8; 72 + 4 + 208];
        data[0] = libc::AF_INET as u8;
        data[1] = 1;
        data[4..6].copy_from_slice(&1234u16.to_be_bytes());
        data[8..12].copy_from_slice(&[127, 0, 0, 1]);
        data[68..72].copy_from_slice(&42u32.to_ne_bytes());
        data[72..74].copy_from_slice(&212u16.to_ne_bytes());
        data[74..76].copy_from_slice(&2u16.to_ne_bytes());
        data[76 + 68..76 + 72].copy_from_slice(&1234u32.to_ne_bytes());
        data[76 + 128..76 + 136].copy_from_slice(&8192u64.to_ne_bytes());
        data[76 + 200..76 + 208].copy_from_slice(&16384u64.to_ne_bytes());
        let row = parse(&data, "tcp").unwrap();
        assert_eq!(row.local, "127.0.0.1:1234");
        assert_eq!(row.received, Some(8192));
        assert_eq!(row.sent, Some(16384));
        assert_eq!(row.rtt_ms, Some(1.234));
        data.truncate(76 + 104);
        data[72..74].copy_from_slice(&108u16.to_ne_bytes());
        let row = parse(&data, "tcp").unwrap();
        assert!(row.sent.is_none());
        assert!(row.received.is_none());
        data[72..74].copy_from_slice(&0u16.to_ne_bytes());
        assert!(parse(&data, "tcp").is_none());
    }
    #[test]
    fn native_dump_finds_owned_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let rows = collect().unwrap();
        assert!(
            rows.iter()
                .any(|r| r.local == address && r.state == "LISTEN")
        );
    }
}
