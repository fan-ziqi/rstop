use rstop::{
    app::{App, View},
    config::Config,
    connections::{Collector, Connection, Sample, apply_tcp_info, parse_proc},
    diagnostics,
    model::{Disk, Gpu, Memory, Snapshot},
    network_tools,
};
use std::{
    net::TcpListener,
    time::{Duration, Instant},
};

fn socket() -> Connection {
    Connection {
        id: "tcp:42:local:remote".into(),
        inode: 42,
        protocol: "tcp".into(),
        local: "127.0.0.1:8080".into(),
        remote: "[::1]:443".into(),
        state: "ESTABLISHED".into(),
        pids: vec![123],
        process_names: vec!["python".into()],
        ..Default::default()
    }
}
#[test]
fn tcp_info_keeps_sockets_separate_and_rates_handle_resets_and_cookie_reuse() {
    let text = "header\n 0: 0100007F:1F90 0100007F:01BB 01 0000000A:00000014 00:0 0 1000 0 42\n";
    let mut rows = parse_proc("tcp", text);
    assert_eq!(rows[0].local, "127.0.0.1:8080");
    assert_eq!(rows[0].send_queue, Some(10));
    assert_eq!(rows[0].receive_queue, Some(20));
    let tcp = "ESTAB 0 0 local remote ino:999 sk:a bytes_sent:999\nESTAB 0 0 local remote ino:42 sk:b bytes_sent:100 bytes_received:200 rtt:2.5/0.2 retrans:0/3";
    apply_tcp_info(&mut rows, tcp);
    assert_eq!(rows[0].received, Some(200));
    assert_eq!(rows[0].rtt_ms, Some(2.5));
    assert_eq!(rows[0].retransmits, Some(3));
    let now = Instant::now();
    let mut collector = Collector::default();
    let mut sample = Sample {
        connections: rows,
        ..Default::default()
    };
    collector.update_rates(&mut sample, now);
    assert_eq!(sample.connections[0].rx_per_sec, None);
    sample.connections[0].received = Some(1200);
    sample.connections[0].sent = Some(600);
    collector.update_rates(&mut sample, now + Duration::from_secs(2));
    assert_eq!(sample.connections[0].rx_per_sec, Some(500.));
    assert_eq!(sample.connections[0].tx_per_sec, Some(250.));
    sample.connections[0].received = Some(1);
    collector.update_rates(&mut sample, now + Duration::from_secs(3));
    assert_eq!(sample.connections[0].rx_per_sec, None);
    sample.connections[0].id.push_str(":new-cookie");
    collector.update_rates(&mut sample, now + Duration::from_secs(4));
    assert_eq!(sample.connections[0].tx_per_sec, None);
    assert_eq!(
        parse_proc("udp", &text.replace(" 01 ", " 07 "))[0].state,
        "BOUND"
    );
}
#[test]
fn native_connection_fallbacks_parse_ipv6_udp_owners_and_states() {
    let rows = rstop::connections::parse_netstat(
        " TCP [::1]:80 [::1]:5000 ESTABLISHED 123\n UDP 0.0.0.0:53 *:* 7\n invalid\n",
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].state, "BOUND");
    assert_eq!(rows[0].pids, vec![123]);
    let rows = rstop::connections::parse_lsof(
        "p42\ncserver\nf12\ntIPv6\nPTCP\nn[::1]:8080->[::1]:443\nTST=ESTABLISHED\nf13\ntIPv4\nPUDP\nn*:5353\n",
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].state, "ESTABLISHED");
    assert_eq!(rows[0].remote, "[::1]:443");
    assert_eq!(rows[1].protocol, "udp");
}
#[test]
fn connection_filters_use_exact_pid_port_and_keep_unknown_counters_unknown() {
    let c = socket();
    for query in [
        "port:443 pid:123 process:PYTHON",
        "host:::1 !state:LISTEN",
        "[::1]:443",
        "proto:tcp",
    ] {
        assert!(network_tools::matches(&c, query), "{query}");
    }
    for query in ["port:44", "pid:12", "proto:udp", "!process:python"] {
        assert!(!network_tools::matches(&c, query), "{query}");
    }
    assert!(
        network_tools::Capture {
            output: std::path::PathBuf::from("Cargo.toml"),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    let capture = network_tools::Capture {
        filter: "host example.com ; touch /tmp/should-not-run".into(),
        ..Default::default()
    };
    assert!(capture.args().contains(&"--".to_string()));
    assert_eq!(capture.args().last().unwrap(), "/tmp/should-not-run"); // literal BPF tokens, never shell execution
    assert!(network_tools::check_target("https://user:secret@example.com").is_err());
    assert!(network_tools::check_target("localhost:0").is_err());
    assert!(network_tools::check_target("localhost:65536").is_err());
}
#[test]
fn tcp_service_check_contacts_only_an_owned_listener_and_explains_the_boundary() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let report = network_tools::check_target(&address.to_string()).unwrap();
    assert!(report.contains("connected in"));
    assert!(report.contains("not that the application is healthy"));
    drop(listener);
    let report = network_tools::check_target(&address.to_string()).unwrap();
    assert!(!report.contains(" · connected in"));
}
fn snapshot() -> Snapshot {
    Snapshot {
        timestamp: 10,
        memory: Memory {
            total: 1000,
            available: 800,
            used: 900,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn diagnosis_is_scoped_reports_evidence_and_does_not_label_cache_or_unknown_as_a_leak() {
    let mut app = App::new(Config {
        alert_hold_seconds: 0,
        ..Default::default()
    });
    app.ingest(snapshot());
    assert!(diagnostics::findings(&app).is_empty());
    let mut s = snapshot();
    s.timestamp = 11;
    s.memory.available = 5;
    s.disks = vec![Disk {
        mount: "/owned".into(),
        total: 1000,
        used: 950,
        available: 50,
        ..Default::default()
    }];
    s.gpus = vec![Gpu {
        memory_total: Some(1000),
        memory_used: Some(950),
        ..Default::default()
    }];
    app.ingest(s);
    app.set_view(View::Storage);
    let report = diagnostics::report(&app);
    assert!(report.contains("/owned is nearly full"));
    assert!(report.contains("Evidence:"));
    assert!(!report.contains("LARGEST"));
    assert!(!report.contains("GPU 0 memory is nearly full"));
    assert!(report.contains("Storage /owned nearly full"));
    let mut s = snapshot();
    s.timestamp = 12;
    app.ingest(s);
    assert!(diagnostics::report(&app).contains("RECOVERED"));
    for legacy in [View::Explorer, View::History, View::Events] {
        assert!(!app.views().contains(&legacy));
    }
}
#[test]
fn connection_selection_survives_rate_resort_and_all_network_modes_render() {
    let mut app = App::new(Config::default());
    let mut s = snapshot();
    let mut a = socket();
    a.tx_per_sec = Some(0.);
    let mut b = socket();
    b.id = "second".into();
    b.rx_per_sec = Some(1000.);
    b.tx_per_sec = Some(0.);
    s.connections.connections = vec![a, b];
    app.ingest(s.clone());
    app.set_view(View::Network);
    app.selected = 1;
    app.remember_selection();
    s.connections.connections[0].rx_per_sec = Some(2000.);
    app.ingest(s);
    assert_eq!(
        app.filtered_connections()[app.selected].id,
        "tcp:42:local:remote"
    );
    for (w, h) in [(60, 18), (80, 24), (120, 36), (160, 48)] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        for mode in [0, 1] {
            app.set_network_section(mode);
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        }
    }
}
#[cfg(unix)]
#[test]
fn folder_report_counts_owned_files_without_following_symlinks() {
    let path = std::env::temp_dir().join(format!("rstop-folder-ops-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join("logs")).unwrap();
    std::fs::write(path.join("logs/large.log"), vec![0; 2048]).unwrap();
    std::os::unix::fs::symlink("/", path.join("leave-this-tree")).unwrap();
    let report = diagnostics::folder_report(&path).unwrap();
    assert!(report.contains("2.0 KiB"));
    assert!(report.contains("large.log"));
    assert!(report.contains("1 symlinks/other filesystems skipped"));
    std::fs::remove_dir_all(path).unwrap();
}
#[cfg(target_os = "linux")]
#[test]
fn live_connection_collection_identifies_an_owned_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let sample = Collector::default().collect();
    let row = sample
        .connections
        .iter()
        .find(|c| c.local == endpoint)
        .expect("owned listener must be visible");
    assert_eq!(row.state, "LISTEN");
    assert!(row.pids.contains(&std::process::id()));
    assert!(!row.process_names.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn live_tcp_payload_rates_follow_an_owned_transfer_when_ss_is_available() {
    use std::io::{Read, Write};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    let endpoint = client.local_addr().unwrap().to_string();
    peer.write_all(&[7u8; 1024]).unwrap();
    client.read_exact(&mut [0u8; 1024]).unwrap();
    let mut collector = Collector::default();
    let first = collector.collect();
    if first
        .notes
        .iter()
        .any(|n| n.starts_with("TCP rates unavailable:"))
    {
        return;
    }
    peer.write_all(&[8u8; 8192]).unwrap();
    client.read_exact(&mut [0u8; 8192]).unwrap();
    let second = collector.collect();
    let a = first
        .connections
        .iter()
        .find(|c| c.local == endpoint)
        .unwrap();
    let b = second
        .connections
        .iter()
        .find(|c| c.local == endpoint)
        .unwrap();
    assert_eq!(b.received.unwrap() - a.received.unwrap(), 8192);
    assert!(b.rx_per_sec.is_some_and(|v| v > 0.));
}
#[test]
fn lsof_rows_keep_queue_sizes_and_drop_duplicate_descriptors() {
    let rows = rstop::connections::parse_lsof(
        "p7\ncapp\nf3\nd0xabc\ntIPv4\nPTCP\nn1.1.1.1:1->2.2.2.2:2\nTST=ESTABLISHED\nTQR=5\nTQS=9\nf4\nd0xabc\ntIPv4\nPTCP\nn1.1.1.1:1->2.2.2.2:2\nTST=ESTABLISHED\nTQR=5\nTQS=9\n",
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].receive_queue, Some(5));
    assert_eq!(rows[0].send_queue, Some(9));
}

#[test]
fn lsof_keeps_distinct_sockets_bound_to_the_same_endpoint() {
    let rows = rstop::connections::parse_lsof(
        "p7\ncapp\nf3\nd0xabc\nPUDP\nn*:5353\nTQR=5\nf4\nd0xdef\nPUDP\nn*:5353\nTQR=9\nf5\nPUDP\nn*:5353\nd0xabc\nTQR=5\n",
    );
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].id, rows[1].id);
    assert_eq!(rows[0].receive_queue, Some(5));
    assert_eq!(rows[1].receive_queue, Some(9));
}

#[test]
fn lsof_keeps_unknown_socket_identities_and_resets_each_file_record() {
    let rows = rstop::connections::parse_lsof(
        "p7\ncapp\nf3\nPTCP\nn*:5353\nTST=LISTEN\nTQR=5\nf4\nd0x0\nPUDP\nn*:5353\nf5\nPUDP\nn*:5353\np8\ncother\nf3\nn*:9999\nf4\nPUDP\nn*:5353\n",
    );
    assert_eq!(rows.len(), 4);
    let ids: std::collections::HashSet<_> = rows.iter().map(|row| &row.id).collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(rows[1].state, "BOUND");
    assert_eq!(rows[1].receive_queue, None);
    assert_eq!(rows[3].pids, vec![8]);
    assert_eq!(rows[3].process_names, vec!["other"]);
}

#[cfg(target_os = "macos")]
#[test]
fn live_lsof_identifies_duplicated_socket_descriptors() {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let duplicate = socket.try_clone().unwrap();
    let address = socket.local_addr().unwrap().to_string();
    let pid = std::process::id().to_string();
    let output = rstop::discovery::command(
        "lsof",
        &["-a", "-p", &pid, "-nP", "-iUDP", "-FpcfdnPtT"],
        Duration::from_secs(3),
    )
    .unwrap();
    let rows = rstop::connections::parse_lsof(&output);
    let owned: Vec<_> = rows.iter().filter(|row| row.local == address).collect();
    assert_eq!(owned.len(), 1, "{output}");
    assert!(owned[0].id.contains(":socket:"), "{output}");
    drop(duplicate);
}
