//! Explicit, bounded network investigations. Arguments never go through a shell.
use crate::{connections::Connection, discovery::command, model::bytes};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub fn matches(row: &Connection, query: &str) -> bool {
    query.split_whitespace().all(|word| {
        let (negated, word) = word
            .strip_prefix('!')
            .map(|s| (true, s))
            .unwrap_or((false, word));
        let (field, value) = word.split_once(':').unwrap_or(("", word));
        let value = value.to_lowercase();
        let found = match field {
            "tag" => crate::tags::matches(&row.tags, &value),
            "pid" => value
                .parse::<u32>()
                .ok()
                .is_some_and(|pid| row.pids.contains(&pid)),
            "port" => [&row.local, &row.remote]
                .iter()
                .any(|s| s.rsplit_once(':').is_some_and(|(_, p)| p == value)),
            "state" => row.state.to_lowercase().contains(&value),
            "process" | "name" => row
                .process_names
                .iter()
                .any(|s| s.to_lowercase().contains(&value)),
            "proto" => row.protocol.to_lowercase().contains(&value),
            "host" => format!("{} {}", row.local, row.remote)
                .to_lowercase()
                .contains(&value),
            _ => format!(
                "{} {} {} {} {:?} {} {}",
                row.protocol,
                row.local,
                row.remote,
                row.state,
                row.pids,
                row.process_names.join(" "),
                crate::tags::text(&row.tags)
            )
            .to_lowercase()
            .contains(&word.to_lowercase()),
        };
        found != negated
    })
}

pub fn check_target(target: &str) -> Result<String> {
    let target = target.trim();
    ensure!(
        !target.is_empty() && target.len() <= 2048 && !target.chars().any(char::is_control),
        "Enter a website URL or host:port, for example https://example.com or localhost:3000"
    );
    if target.starts_with("https://") || target.starts_with("http://") {
        ensure!(
            !target.contains('@'),
            "Use a URL without embedded credentials"
        );
        let output = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let mut body = format!(
            "WEBSITE CHECK · {target}\nHEAD request; certificate verification enabled. No redirects followed.\nDNS/TCP/TLS are cumulative seconds since request start; total is time to headers.\n\n"
        );
        let proxy_keys: Vec<_> = [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ]
        .into_iter()
        .filter(|key| std::env::var(key).is_ok_and(|s| !s.is_empty()))
        .collect();
        for direct in [true, false] {
            if !direct && proxy_keys.is_empty() {
                continue;
            }
            let mut args = vec![
                "--disable",
                "--silent",
                "--show-error",
                "--globoff",
                "--head",
                "--proto",
                "=http,https",
                "--connect-timeout",
                "3",
                "--max-time",
                "8",
                "--output",
                output,
                "--write-out",
                "HTTP %{http_code}\nRemote %{remote_ip}:%{remote_port}\nDNS %{time_namelookup}s\nTCP %{time_connect}s\nTLS %{time_appconnect}s\nFirst byte %{time_starttransfer}s\nTotal %{time_total}s\n",
            ];
            if direct {
                args.extend(["--noproxy", "*"]);
            }
            args.extend(["--url", target]);
            body.push_str(if direct {
                "DIRECT (bypass environment proxy)\n"
            } else {
                "ENVIRONMENT PROXY (respects NO_PROXY)\n"
            });
            match command("curl", &args, Duration::from_secs(9)) {
                Ok(output) => {
                    body.push_str(&output);
                    body.push_str("A response proves this HTTP endpoint was reached; 4xx/5xx indicate an application/access error. 405 can mean HEAD is unsupported.\n");
                }
                Err(error) => body.push_str(&format!("Failed: {error}\nCould not complete this request; check the reported DNS, connection, timeout or certificate error.\n")),
            }
            body.push('\n');
        }
        body.push_str(&format!("Proxy variables set: {} (values hidden).\nIf direct fails but proxy works, check your proxy/route requirements. If both fail, use Network tools > route/DNS settings.\n", if proxy_keys.is_empty() { "none".into() } else { proxy_keys.join(", ") }));
        return Ok(body);
    }
    ensure!(
        !target.contains('/') && !target.starts_with('-'),
        "Use http(s):// for websites, or host:port for TCP services"
    );
    let (host, port) = target
        .rsplit_once(':')
        .context("Include the port, for example localhost:5432 or [::1]:3000")?;
    ensure!(
        !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0),
        "The TCP port must be between 1 and 65535"
    );
    let began = Instant::now();
    let address = target.to_string();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        use std::net::ToSocketAddrs;
        let result = address
            .to_socket_addrs()
            .map(|a| a.take(4).collect::<Vec<_>>());
        let _ = tx.send(result);
    });
    let addresses = rx
        .recv_timeout(Duration::from_secs(3))
        .context("DNS lookup did not finish within 3 seconds")??;
    ensure!(!addresses.is_empty(), "DNS returned no addresses");
    let mut body = format!(
        "TCP SERVICE CHECK · {target}\nDNS resolved in {:.1} ms. Testing up to four addresses directly (no proxy).\n\n",
        began.elapsed().as_secs_f64() * 1000.
    );
    for address in addresses {
        let began = Instant::now();
        match std::net::TcpStream::connect_timeout(&address, Duration::from_millis(1000)) {
            Ok(_) => body.push_str(&format!(
                "{address} · connected in {:.1} ms\n",
                began.elapsed().as_secs_f64() * 1000.
            )),
            Err(error) => body.push_str(&format!("{address} · {error}\n")),
        }
    }
    body.push_str("\nConnected means TCP accepted a connection, not that the application is healthy. Refused usually means no listener or an active rejection; timeout can be a route, firewall or unresponsive peer.\nUse port:<number> in the connection filter to find local owners.\n");
    Ok(body)
}

pub fn settings() -> String {
    let mut body = "NETWORK ROUTES & DNS\nRoutes choose where packets go; DNS translates a name into an address.\n\n".to_string();
    #[cfg(target_os = "linux")]
    {
        for (label, args) in [
            ("IPv4 routes", vec!["-4", "route", "show"]),
            ("IPv6 routes", vec!["-6", "route", "show"]),
            ("Policy rules (VPN/proxy routing)", vec!["rule", "show"]),
        ] {
            body.push_str(&format!(
                "{label}\n{}\n",
                command("ip", &args, Duration::from_secs(2)).unwrap_or_else(|e| e.to_string())
            ));
        }
        body.push_str(&format!(
            "Resolver configuration\n{}\n",
            std::fs::read_to_string("/etc/resolv.conf").unwrap_or_else(|e| e.to_string())
        ));
        body.push_str(&format!(
            "Per-interface DNS (when systemd-resolved is installed)\n{}\n",
            command(
                "resolvectl",
                &["status", "--no-pager"],
                Duration::from_secs(2)
            )
            .unwrap_or_else(|e| e.to_string())
        ));
    }
    #[cfg(target_os = "macos")]
    {
        body.push_str(
            &command("netstat", &["-rn"], Duration::from_secs(2)).unwrap_or_else(|e| e.to_string()),
        );
        body.push_str(
            &command("scutil", &["--dns"], Duration::from_secs(2))
                .unwrap_or_else(|e| e.to_string()),
        );
    }
    #[cfg(windows)]
    {
        body.push_str(
            &command("route", &["print"], Duration::from_secs(2)).unwrap_or_else(|e| e.to_string()),
        );
        body.push_str(
            &command("ipconfig", &["/all"], Duration::from_secs(2))
                .unwrap_or_else(|e| e.to_string()),
        );
    }
    body
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capture {
    pub interface: String,
    pub filter: String,
    pub output: PathBuf,
    pub seconds: u64,
    pub packets: u32,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            interface: if cfg!(target_os = "linux") {
                "any"
            } else {
                "en0"
            }
            .into(),
            filter: "tcp or udp or icmp or icmp6".into(),
            output: PathBuf::from(format!("rstop-{}.pcap", crate::collector::unix_time())),
            seconds: 10,
            packets: 2000,
        }
    }
}
impl Capture {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            cfg!(unix),
            "Packet capture currently requires tcpdump on Linux/macOS; Windows connections and service checks remain available"
        );
        ensure!(
            !self.interface.is_empty()
                && self.interface.len() <= 128
                && !self.interface.chars().any(char::is_control),
            "Choose a valid interface name"
        );
        ensure!(
            self.filter.len() <= 1024 && !self.filter.chars().any(char::is_control),
            "Capture filter must be at most 1024 characters"
        );
        ensure!(
            (1..=60).contains(&self.seconds) && (1..=10000).contains(&self.packets),
            "Capture limits: 1..60 seconds, 1..10000 packets"
        );
        ensure!(
            !self.output.as_os_str().is_empty() && !self.output.exists(),
            "Choose a new output file; existing files are never overwritten"
        );
        Ok(())
    }
    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = [
            "-p",
            "-n",
            "-U",
            "-i",
            &self.interface,
            "-s",
            "128",
            "-c",
            &self.packets.to_string(),
            "-w",
            "-",
            "--",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        args.extend(self.filter.split_whitespace().map(String::from));
        args
    }
    pub fn retry_command(&self) -> String {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\"'\"'"));
        let executable = std::env::current_exe()
            .map(|p| quote(&p.to_string_lossy()))
            .unwrap_or_else(|_| "rstop".into());
        format!(
            "{executable} network capture {} --interface {} --filter {} --seconds {} --packets {}",
            quote(&self.output.to_string_lossy()),
            quote(&self.interface),
            quote(&self.filter),
            self.seconds,
            self.packets
        )
    }
    pub fn preview(&self) -> String {
        format!(
            "Capture on {} for up to {}s / {} packets\nFilter: {}\nSave: {}\n\nOnly the first 128 bytes of each packet are saved; those bytes can still contain private data.\nNo automatic privilege elevation. A new private PCAP file is created.\n\nCommand: {} (tcpdump backend)\n\n[y/Enter] Confirm · [n/Esc] Cancel",
            self.interface,
            self.seconds,
            self.packets,
            self.filter,
            self.output.display(),
            self.retry_command()
        )
    }
    pub fn run(&self) -> Result<String> {
        self.run_with_program("tcpdump")
    }
    fn run_with_program(&self, program: &str) -> Result<String> {
        self.validate()?;
        #[cfg(unix)]
        {
            use std::{
                fs::OpenOptions,
                io::Read,
                os::unix::{fs::OpenOptionsExt, process::CommandExt},
                process::{Command, Stdio},
                sync::atomic::Ordering,
                thread,
            };
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&self.output)?;
            let result = Command::new(program)
                .args(self.args())
                .process_group(0)
                .env("LC_ALL", "C")
                .stdin(Stdio::null())
                .stdout(Stdio::from(file))
                .stderr(Stdio::piped())
                .spawn();
            let mut child = match result {
                Ok(child) => child,
                Err(error) => {
                    let _ = std::fs::remove_file(&self.output);
                    return Err(error).context("Install tcpdump to capture packets");
                }
            };
            let stderr = child
                .stderr
                .take()
                .context("Cannot read tcpdump diagnostics")?;
            let reader = thread::spawn(move || {
                let mut bytes = vec![];
                let _ = stderr.take(65536).read_to_end(&mut bytes);
                String::from_utf8_lossy(&bytes).into_owned()
            });
            let began = Instant::now();
            let mut stopped = None;
            let status = loop {
                if let Some(status) = child.try_wait()? {
                    break status;
                }
                if (began.elapsed() >= Duration::from_secs(self.seconds)
                    || crate::STOP.load(Ordering::Relaxed))
                    && stopped.is_none()
                {
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGINT);
                    }
                    stopped = Some(Instant::now());
                }
                if stopped.is_some_and(|at| at.elapsed() > Duration::from_secs(1)) {
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.kill();
                    break child.wait()?;
                }
                thread::sleep(Duration::from_millis(20));
            };
            let diagnostics = reader.join().unwrap_or_default();
            let length = std::fs::metadata(&self.output)?.len();
            if length < 24 {
                let _ = std::fs::remove_file(&self.output);
                let denied = diagnostics.to_lowercase();
                let next = if denied.contains("permission")
                    || denied.contains("permitted")
                    || denied.contains("access denied")
                {
                    format!(
                        "Packet capture needs administrator permission on {}.\nClose rstop and run this scoped command in your terminal to request permission:\n  sudo {}\n\nYour system asks for your password; typing is hidden. Only this capture is elevated. rstop does not store the password.",
                        self.interface,
                        self.retry_command()
                    )
                } else {
                    "Check the interface and packet filter. No valid capture file was created."
                        .into()
                };
                anyhow::bail!("Capture failed: {diagnostics}\n{next}");
            }
            let mut body = format!(
                "Saved {} ({})\nStopped after {:.1}s; tcpdump exit: {}\n{}\n",
                self.output.display(),
                bytes(length as f64),
                began.elapsed().as_secs_f64(),
                status,
                diagnostics
            );
            if length == 24 {
                body.push_str("No packets were captured in this window. Check the interface/filter and generate the traffic you want to inspect.\n");
            }
            if !status.success() {
                body.push_str(
                    "The helper did not exit successfully; the saved capture may be partial.\n",
                );
            }
            body.push_str(
                &read_capture(&self.output)
                    .unwrap_or_else(|e| format!("Cannot preview capture: {e}")),
            );
            Ok(body)
        }
        #[cfg(not(unix))]
        {
            let _ = program;
            anyhow::bail!("Packet capture backend unavailable on this platform")
        }
    }
}
pub fn read_capture(path: &Path) -> Result<String> {
    ensure!(path.is_file(), "Choose a regular PCAP file");
    let canonical = path.canonicalize()?;
    let path = canonical
        .to_str()
        .context("Path must be valid UTF-8 for tcpdump")?;
    let output = command(
        "tcpdump",
        &["-n", "-tttt", "-r", path, "-c", "60"],
        Duration::from_secs(3),
    )?;
    Ok(format!(
        "FIRST 60 PACKETS · headers only\n[S] TCP opening, [S.] opening reply, [R] reset/rejection, [F] closing.\nICMP unreachable reports a delivery failure. Absence of replies in this short sample is not proof of a firewall fault.\nOpen the PCAP in Wireshark for stream/retransmission analysis.\n\n{output}"
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    #[test]
    fn capture_bounds_helper_lifetime_and_preserves_existing_files_and_private_permissions() {
        let dir = std::env::temp_dir().join(format!("rstop-capture-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let script = dir.join("owned-capture-helper");
        // Valid empty little-endian PCAP header. The long-running helper must be reclaimed.
        fs::write(&script,b"#!/bin/sh\nprintf '\\324\\303\\262\\241\\002\\000\\004\\000\\000\\000\\000\\000\\000\\000\\000\\000\\200\\000\\000\\000\\001\\000\\000\\000'\nsleep 20\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let capture = Capture {
            output: dir.join("owned.pcap"),
            seconds: 1,
            ..Default::default()
        };
        let began = Instant::now();
        let report = capture.run_with_program(script.to_str().unwrap()).unwrap();
        assert!(began.elapsed() < Duration::from_secs(5));
        assert!(report.contains("Saved"));
        assert_eq!(
            fs::metadata(&capture.output).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let original = fs::read(&capture.output).unwrap();
        assert_eq!(original.len(), 24);
        assert!(capture.run_with_program(script.to_str().unwrap()).is_err());
        assert_eq!(fs::read(&capture.output).unwrap(), original);
        let denied = Capture {
            output: dir.join("denied.pcap"),
            ..Default::default()
        };
        fs::write(
            &script,
            b"#!/bin/sh\necho 'permission denied' >&2\nexit 1\n",
        )
        .unwrap();
        assert!(
            denied
                .run_with_program(script.to_str().unwrap())
                .unwrap_err()
                .to_string()
                .contains("permission denied")
        );
        assert!(!denied.output.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
