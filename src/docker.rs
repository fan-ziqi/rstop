//! Local Unix-socket Docker Engine reads, with CLI fallback in discovery.
use crate::{
    collector::unix_time,
    model::{Container, duration},
};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::net::UnixStream,
    },
    time::{Duration, Instant},
};

fn connect(path: &str, end: Instant) -> Result<UnixStream> {
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    ensure!(
        !path.as_bytes().contains(&0) && path.len() < address.sun_path.len(),
        "Invalid Docker socket path"
    );
    address.sun_family = libc::AF_UNIX as _;
    for (dest, src) in address.sun_path.iter_mut().zip(path.bytes()) {
        *dest = src as _;
    }
    let length =
        (std::mem::offset_of!(libc::sockaddr_un, sun_path) + path.len() + 1) as libc::socklen_t;
    #[cfg(target_os = "macos")]
    {
        address.sun_len = length as _;
    }
    #[cfg(target_os = "linux")]
    let flags = libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
    #[cfg(not(target_os = "linux"))]
    let flags = libc::SOCK_STREAM;
    let raw = unsafe { libc::socket(libc::AF_UNIX, flags, 0) };
    if raw < 0 {
        return Err(io::Error::last_os_error().into());
    }
    let stream = unsafe { UnixStream::from_raw_fd(raw) };
    stream.set_nonblocking(true)?;
    #[cfg(not(target_os = "linux"))]
    if unsafe { libc::fcntl(raw, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error().into());
    }
    let result = unsafe { libc::connect(raw, &address as *const _ as _, length) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error.into());
        }
        loop {
            let left = end.checked_duration_since(Instant::now()).ok_or_else(|| {
                io::Error::new(io::ErrorKind::TimedOut, "Docker connect deadline")
            })?;
            let mut descriptor = libc::pollfd {
                fd: raw,
                events: libc::POLLOUT,
                revents: 0,
            };
            let ready = unsafe {
                libc::poll(
                    &mut descriptor,
                    1,
                    left.as_millis().min(i32::MAX as u128).max(1) as i32,
                )
            };
            if ready < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            ensure!(ready > 0, "Docker socket connect failed or timed out");
            let mut error = 0i32;
            let mut size = std::mem::size_of_val(&error) as libc::socklen_t;
            if unsafe {
                libc::getsockopt(
                    stream.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    &mut error as *mut _ as _,
                    &mut size,
                )
            } < 0
            {
                return Err(io::Error::last_os_error().into());
            }
            if error != 0 {
                return Err(io::Error::from_raw_os_error(error).into());
            }
            break;
        }
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}
const LIMIT: usize = 16 * 1024 * 1024;
struct Deadline {
    stream: UnixStream,
    end: Instant,
}
impl Read for Deadline {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            let left = self
                .end
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Docker read deadline"))?;
            let mut descriptor = libc::pollfd {
                fd: self.stream.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe {
                libc::poll(
                    &mut descriptor,
                    1,
                    left.as_millis().clamp(1, i32::MAX as u128) as i32,
                )
            };
            if ready < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if ready == 0 {
                continue;
            }
            match self.stream.read(bytes) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                    ) => {}
                result => return result,
            }
        }
    }
}
fn line(reader: &mut impl BufRead) -> Result<String> {
    let mut value = String::new();
    let count = reader.take(65537).read_line(&mut value)?;
    ensure!(
        count > 0 && count <= 65536 && value.ends_with('\n'),
        "Invalid Docker HTTP line"
    );
    Ok(value)
}
fn body(reader: &mut impl BufRead) -> Result<Vec<u8>> {
    let status = line(reader)?;
    ensure!(
        status.split_whitespace().nth(1) == Some("200"),
        "Docker API: {}",
        status.trim()
    );
    let mut length = None;
    let mut chunked = false;
    let mut header_bytes = status.len();
    loop {
        let header = line(reader)?;
        header_bytes += header.len();
        ensure!(header_bytes <= 65536, "Docker HTTP headers too large");
        if header == "\r\n" {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = Some(value.trim().parse::<usize>()?);
            }
            if key.eq_ignore_ascii_case("transfer-encoding") {
                chunked = value
                    .split(',')
                    .any(|s| s.trim().eq_ignore_ascii_case("chunked"));
            }
        }
    }
    let mut data = Vec::new();
    if chunked {
        loop {
            let header = line(reader)?;
            let count = usize::from_str_radix(header.trim().split(';').next().unwrap_or(""), 16)?;
            if count == 0 {
                break;
            }
            ensure!(
                count <= LIMIT.saturating_sub(data.len()),
                "Docker response too large"
            );
            let old = data.len();
            data.resize(old + count, 0);
            reader.read_exact(&mut data[old..])?;
            let mut end = [0; 2];
            reader.read_exact(&mut end)?;
            ensure!(end == *b"\r\n", "Invalid Docker chunk terminator");
        }
    } else if let Some(count) = length {
        ensure!(count <= LIMIT, "Docker response too large");
        data.resize(count, 0);
        reader.read_exact(&mut data)?;
    } else {
        reader.take(LIMIT as u64 + 1).read_to_end(&mut data)?;
        ensure!(data.len() <= LIMIT, "Docker response too large");
    }
    Ok(data)
}
fn get(socket: &str, path: &str, end: Instant) -> Result<Value> {
    ensure!(
        path.starts_with('/') && !path.bytes().any(|b| b.is_ascii_control()),
        "Invalid Docker API path"
    );
    let mut stream = connect(socket, end).context("Connect Docker socket")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .context("Set Docker write timeout")?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .context("Write Docker request")?;
    // Poll against the shared deadline on both Linux and macOS instead of
    // repeatedly changing the socket's receive timeout.
    stream
        .set_nonblocking(true)
        .context("Set Docker read mode")?;
    let data =
        body(&mut BufReader::new(Deadline { stream, end })).context("Read Docker response")?;
    serde_json::from_slice(&data).context("Parse Docker response")
}
fn created_at(seconds: u64) -> String {
    let value = seconds as libc::time_t;
    let mut time = unsafe { std::mem::zeroed::<libc::tm>() };
    let mut buffer = [0u8; 64];
    unsafe {
        if libc::gmtime_r(&value, &mut time).is_null() {
            return seconds.to_string();
        }
        let size = libc::strftime(
            buffer.as_mut_ptr() as _,
            buffer.len(),
            c"%Y-%m-%dT%H:%M:%SZ".as_ptr(),
            &time,
        );
        String::from_utf8_lossy(&buffer[..size]).into_owned()
    }
}
fn apply_stats(c: &mut Container, v: &Value) {
    let cpu = &v["cpu_stats"];
    let previous = &v["precpu_stats"];
    let cpus = cpu["online_cpus"].as_u64().filter(|n| *n > 0).or_else(|| {
        cpu["cpu_usage"]["percpu_usage"]
            .as_array()
            .map(|a| a.len() as u64)
    });
    c.cpu = cpu["cpu_usage"]["total_usage"]
        .as_u64()
        .zip(previous["cpu_usage"]["total_usage"].as_u64())
        .and_then(|(a, b)| a.checked_sub(b))
        .zip(
            cpu["system_cpu_usage"]
                .as_u64()
                .zip(previous["system_cpu_usage"].as_u64())
                .and_then(|(a, b)| a.checked_sub(b)),
        )
        .zip(cpus)
        .and_then(|((process, system), cores)| {
            (system > 0).then_some(process as f64 / system as f64 * cores as f64 * 100.)
        });
    let memory = &v["memory_stats"];
    let cache = memory["stats"]["total_inactive_file"]
        .as_u64()
        .or_else(|| memory["stats"]["inactive_file"].as_u64())
        .unwrap_or(0);
    c.memory = memory["usage"]
        .as_u64()
        .map(|used| if cache < used { used - cache } else { used });
    c.memory_limit = memory["limit"].as_u64();
    c.pids = v["pids_stats"]["current"].as_u64();
    if let Some(networks) = v["networks"].as_object() {
        c.network_rx = Some(
            networks
                .values()
                .filter_map(|v| v["rx_bytes"].as_u64())
                .fold(0, u64::saturating_add),
        );
        c.network_tx = Some(
            networks
                .values()
                .filter_map(|v| v["tx_bytes"].as_u64())
                .fold(0, u64::saturating_add),
        );
    }
    if let Some(values) = v["blkio_stats"]["io_service_bytes_recursive"].as_array() {
        c.block_read = Some(
            values
                .iter()
                .filter(|v| {
                    v["op"]
                        .as_str()
                        .is_some_and(|op| op.eq_ignore_ascii_case("read"))
                })
                .filter_map(|v| v["value"].as_u64())
                .fold(0, u64::saturating_add),
        );
        c.block_write = Some(
            values
                .iter()
                .filter(|v| {
                    v["op"]
                        .as_str()
                        .is_some_and(|op| op.eq_ignore_ascii_case("write"))
                })
                .filter_map(|v| v["value"].as_u64())
                .fold(0, u64::saturating_add),
        );
    }
}
pub(crate) struct Inventory {
    pub containers: Vec<Container>,
    pub notes: Vec<String>,
}
pub(crate) fn discover_with_updates(
    endpoint: &str,
    mut publish: impl FnMut(&Inventory),
) -> Result<Inventory> {
    let socket = endpoint
        .strip_prefix("unix://")
        .ok_or_else(|| anyhow::anyhow!("Not a Unix-socket Docker endpoint"))?;
    let end = Instant::now() + Duration::from_secs(5);
    let list = get(socket, "/containers/json?all=true", end)?;
    let list = list
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid Docker container list"))?;
    ensure!(list.len() <= 20_000, "Container list too large");
    let mut containers = Vec::with_capacity(list.len());
    for v in list {
        let id = v["Id"].as_str().unwrap_or("");
        ensure!(
            (12..=64).contains(&id.len()) && id.bytes().all(|c| c.is_ascii_hexdigit()),
            "Invalid Docker container ID"
        );
        let created = v["Created"].as_u64();
        let ports = v["Ports"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .map(|p| {
                        let private = p["PrivatePort"].as_u64().unwrap_or(0);
                        let kind = p["Type"].as_str().unwrap_or("?");
                        if let Some(public) = p["PublicPort"].as_u64() {
                            let ip = p["IP"].as_str().unwrap_or("0.0.0.0");
                            if ip.contains(':') {
                                format!("[{ip}]:{public}->{private}/{kind}")
                            } else {
                                format!("{ip}:{public}->{private}/{kind}")
                            }
                        } else {
                            format!("{private}/{kind}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        containers.push(Container {
            id: id.into(),
            runtime: "docker".into(),
            name: v["Names"]
                .as_array()
                .and_then(|v| v.first())
                .and_then(Value::as_str)
                .unwrap_or(id)
                .trim_start_matches('/')
                .into(),
            image: v["Image"].as_str().unwrap_or("").into(),
            state: v["State"].as_str().unwrap_or("").into(),
            status: v["Status"].as_str().unwrap_or("").into(),
            ports,
            command: v["Command"].as_str().unwrap_or("").into(),
            created: created.map(created_at).unwrap_or_default(),
            running_for: created
                .map(|s| duration(unix_time().saturating_sub(s)))
                .unwrap_or_default(),
            ..Default::default()
        });
    }
    let mut inventory = Inventory {
        containers,
        notes: Vec::new(),
    };
    publish(&inventory);
    // Bound concurrency; the Engine supplies the same pair of CPU samples used by docker stats.
    let mut notes = Vec::new();
    for group in inventory.containers.chunks_mut(8) {
        if Instant::now() >= end {
            notes.push("Docker statistics reached the query deadline".into());
            break;
        }
        let errors = std::thread::scope(|scope| {
            let jobs: Vec<_> = group
                .iter_mut()
                .filter(|c| c.state == "running")
                .map(|c| {
                    scope.spawn(move || {
                        let stats = get(
                            socket,
                            &format!("/containers/{}/stats?stream=false", c.id),
                            end,
                        )?;
                        apply_stats(c, &stats);
                        Ok::<_, anyhow::Error>(())
                    })
                })
                .collect();
            jobs.into_iter()
                .filter_map(|job| match job.join() {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(format!("Docker statistics: {error:#}")),
                    Err(_) => Some("Docker reader failed".into()),
                })
                .collect::<Vec<_>>()
        });
        notes.extend(errors);
        notes.truncate(8);
    }
    inventory.notes = notes;
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalled_socket_read_obeys_the_deadline() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        stream.set_nonblocking(true).unwrap();
        let mut reader = Deadline {
            stream,
            end: Instant::now() + Duration::from_millis(20),
        };
        assert_eq!(
            reader.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn queued_response_remains_readable_after_the_peer_closes() {
        let (stream, mut peer) = UnixStream::pair().unwrap();
        stream.set_nonblocking(true).unwrap();
        peer.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap();
        drop(peer);
        let mut reader = BufReader::new(Deadline {
            stream,
            end: Instant::now() + Duration::from_secs(1),
        });
        assert_eq!(body(&mut reader).unwrap(), b"{}");
    }

    #[test]
    fn http_handles_chunk_extensions_and_rejects_incomplete_or_oversized_data() {
        let data=b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2;ignored=true\r\n{}\r\n0\r\n\r\n";
        assert_eq!(body(&mut &data[..]).unwrap(), b"{}");
        assert!(body(&mut &b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n{}"[..]).is_err());
        assert!(body(&mut &b"HTTP/1.1 200 OK\r\nContent-Length: 999999999\r\n\r\n"[..]).is_err());
    }
    #[test]
    fn stats_preserve_cpu_units_cache_memory_and_missing_counters() {
        let v = serde_json::json!({"cpu_stats":{"cpu_usage":{"total_usage":200},"system_cpu_usage":1000,"online_cpus":8},"precpu_stats":{"cpu_usage":{"total_usage":100},"system_cpu_usage":500},"memory_stats":{"usage":1000,"limit":2000,"stats":{"inactive_file":100}},"networks":{"eth0":{"rx_bytes":20,"tx_bytes":30}},"pids_stats":{"current":2}});
        let mut c = Container::default();
        apply_stats(&mut c, &v);
        assert_eq!(c.cpu, Some(160.));
        assert_eq!(c.memory, Some(900));
        assert_eq!(c.network_rx, Some(20));
        assert!(c.block_read.is_none());
        apply_stats(&mut c, &Value::Null);
        assert!(c.cpu.is_none());
        assert!(c.memory.is_none());
    }

    #[test]
    fn local_socket_discovery_lists_stopped_containers_and_samples_only_running_ones() {
        for _ in 0..32 {
            local_socket_discovery_case();
        }
    }

    fn local_socket_discovery_case() {
        use std::{fs, os::unix::net::UnixListener};
        struct OwnedSocket(std::path::PathBuf);
        impl Drop for OwnedSocket {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let path = std::env::temp_dir().join(format!(
            "rstop-docker-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let owned = OwnedSocket(path);
        let listener = UnixListener::bind(&owned.0).unwrap();
        listener.set_nonblocking(true).unwrap();
        let (listed_tx, listed_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(4);
            let mut paths = Vec::new();
            while paths.len() < 2 && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(v) => v,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }
                let path = request.split_whitespace().nth(1).unwrap().to_owned();
                let response = if path == "/containers/json?all=true" {
                    serde_json::json!([
                        {"Id":"aaaaaaaaaaaa", "Names":["/running"], "State":"running", "Ports":[{"IP":"::", "PublicPort":8080,"PrivatePort":80,"Type":"tcp"}]},
                        {"Id":"bbbbbbbbbbbb", "Names":["/stopped"], "State":"exited"}
                    ])
                } else {
                    assert_eq!(path, "/containers/aaaaaaaaaaaa/stats?stream=false");
                    listed_rx.recv_timeout(Duration::from_secs(1))
                        .expect("the container list must be published before statistics are ready");
                    serde_json::json!({"cpu_stats":{"cpu_usage":{"total_usage":200},"system_cpu_usage":1000,"online_cpus":8},"precpu_stats":{"cpu_usage":{"total_usage":100},"system_cpu_usage":500},"memory_stats":{"usage":1000,"limit":2000,"stats":{"inactive_file":100}}})
                }.to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                    response.len()
                )
                .unwrap();
                paths.push(path);
            }
            paths
        });
        let result = discover_with_updates(&format!("unix://{}", owned.0.display()), |partial| {
            assert_eq!(partial.containers.len(), 2);
            assert!(partial.containers[0].cpu.is_none());
            listed_tx.send(()).unwrap();
        });
        let paths = server.join().unwrap();
        let inventory = result.unwrap();
        assert_eq!(paths.len(), 2);
        assert!(inventory.notes.is_empty(), "{:?}", inventory.notes);
        assert_eq!(inventory.containers.len(), 2);
        assert_eq!(inventory.containers[0].cpu, Some(160.));
        assert_eq!(inventory.containers[0].memory, Some(900));
        assert_eq!(inventory.containers[0].ports, "[::]:8080->80/tcp");
        assert_eq!(inventory.containers[1].state, "exited");
        assert!(inventory.containers[1].cpu.is_none());
    }
}
