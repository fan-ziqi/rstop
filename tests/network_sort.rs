use rstop::{
    app::{App, View},
    config::Config,
    connections::Connection,
    model::{Network, Snapshot},
    network_sort::{ConnectionSort as C, InterfaceSort as I},
};
use std::cmp::Ordering;
fn connection(id: &str, n: u64) -> Connection {
    Connection {
        id: id.into(),
        process_names: vec![if n == 2 { "Alpha" } else { "Zulu" }.into()],
        pids: vec![n as u32],
        protocol: if n == 2 { "tcp" } else { "udp" }.into(),
        local: format!("127.0.0.1:{n}"),
        remote: format!("[::1]:{}", if n == 2 { 80 } else { 443 }),
        state: if n == 2 { "ESTABLISHED" } else { "SYN_SENT" }.into(),
        received: Some(n),
        sent: Some(n),
        rx_per_sec: Some(n as f64),
        tx_per_sec: Some(n as f64),
        rtt_ms: Some(n as f64),
        receive_queue: Some(n),
        send_queue: Some(n),
        retransmits: Some(n),
        ..Default::default()
    }
}
#[test]
fn every_connection_sort_uses_its_own_field_and_unknowns_stay_last_both_directions() {
    let a = connection("a", 2);
    let b = connection("b", 10);
    let unknown = Connection {
        id: "unknown".into(),
        ..Default::default()
    };
    assert_eq!(C::ALL.len(), 18);
    for sort in C::ALL {
        let expected = if matches!(
            sort,
            C::Process | C::State | C::Pid | C::Protocol | C::LocalPort | C::RemotePort
        ) {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        assert_eq!(sort.compare(&a, &b, false), expected, "{sort:?}");
        assert_eq!(
            sort.compare(&a, &b, true),
            expected.reverse(),
            "{sort:?} reverse"
        );
        for reverse in [false, true] {
            assert_eq!(
                sort.compare(&unknown, &a, reverse),
                Ordering::Greater,
                "{sort:?} unknown"
            );
        }
    }
    let mut reset = a.clone();
    reset.rx_per_sec = Some(f64::NAN);
    assert_eq!(C::Download.compare(&reset, &b, false), Ordering::Greater);
    assert_eq!(C::Download.compare(&reset, &b, true), Ordering::Greater);
    assert_eq!(C::Process.cell(&a), "Alpha");
    assert_eq!(C::Pid.cell(&a), "2");
    // Preserve integer precision instead of sorting formatted sizes or f64 counters.
    let mut large = a.clone();
    let mut larger = b.clone();
    large.received = Some((1 << 54) + 1);
    larger.received = Some((1 << 54) + 2);
    assert_eq!(
        C::Received.compare(&large, &larger, false),
        Ordering::Greater
    );
}
#[test]
fn interface_sorts_use_numeric_counters_and_keep_missing_speed_and_drops_last() {
    let a = Network {
        name: "a".into(),
        state: "down".into(),
        address: "127.0.0.1".into(),
        rx_per_sec: 2.,
        tx_per_sec: 2.,
        rx_total: 2,
        tx_total: 2,
        errors: 2,
        drops: Some(2),
        speed_mbps: Some(2),
        mtu: 2,
        rx_packets_per_sec: Some(1.),
        tx_packets_per_sec: Some(1.),
        rx_packets: 2,
        tx_packets: 2,
        ..Default::default()
    };
    let b = Network {
        name: "b".into(),
        state: "up".into(),
        address: "127.0.0.2".into(),
        rx_per_sec: 10.,
        tx_per_sec: 10.,
        rx_total: 10,
        tx_total: 10,
        errors: 10,
        drops: Some(10),
        speed_mbps: Some(10),
        mtu: 10,
        rx_packets_per_sec: Some(2.),
        tx_packets_per_sec: Some(2.),
        rx_packets: 10,
        tx_packets: 10,
        ..Default::default()
    };
    assert_eq!(I::ALL.len(), 17);
    for sort in I::ALL {
        let expected = if matches!(sort, I::Name | I::State | I::Address) {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        assert_eq!(sort.compare(&a, &b, false), expected, "{sort:?}");
        assert_eq!(sort.compare(&a, &b, true), expected.reverse(), "{sort:?}");
    }
    let unknown = Network {
        drops: None,
        speed_mbps: None,
        ..a.clone()
    };
    for reverse in [false, true] {
        for sort in [I::Drops, I::Speed] {
            assert_eq!(sort.compare(&unknown, &b, reverse), Ordering::Greater);
        }
    }
}
#[test]
fn sorting_preserves_explicit_connection_and_interface_selection_and_filtering() {
    let mut app = App::new(Config::default());
    let a = connection("a", 2);
    let b = connection("b", 10);
    let interfaces = vec![
        Network {
            name: "a".into(),
            rx_per_sec: 2.,
            ..Default::default()
        },
        Network {
            name: "b".into(),
            rx_per_sec: 10.,
            ..Default::default()
        },
    ];
    app.ingest(Snapshot {
        timestamp: 10,
        connections: rstop::connections::Sample {
            connections: vec![a, b],
            ..Default::default()
        },
        networks: interfaces,
        ..Default::default()
    });
    app.set_view(View::Network);
    app.selected = 1;
    app.remember_selection();
    let id = app.filtered_connections()[1].id.clone();
    app.set_network_sort(C::Pid.index(), false);
    assert_eq!(app.filtered_connections()[app.selected].id, id);
    app.set_network_sort(C::Pid.index(), true);
    assert_eq!(app.filtered_connections()[app.selected].id, id);
    app.set_network_section(1);
    app.selected = 1;
    app.remember_selection();
    app.set_network_sort(I::Download.index(), false);
    assert_eq!(app.filtered_networks()[app.selected].name, "b");
    app.filter = "a".into();
    assert_eq!(app.filtered_networks().len(), 1);
}
#[test]
fn pid_and_process_have_distinct_clickable_headers_and_selected_sort_metrics_remain_visible() {
    let mut app = App::new(Config::default());
    app.ingest(Snapshot {
        timestamp: 1,
        connections: rstop::connections::Sample {
            connections: vec![connection("a", 2)],
            ..Default::default()
        },
        ..Default::default()
    });
    app.set_view(View::Network);
    for (w, h) in [(60, 18), (80, 24), (120, 36), (160, 48)] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        for sort in C::ALL {
            app.set_network_sort(sort.index(), false);
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            assert!(
                app.network_sort_headers
                    .iter()
                    .any(|(_, index)| *index == C::Process.index())
            );
            assert!(
                app.network_sort_headers
                    .iter()
                    .any(|(_, index)| *index == C::Pid.index())
            );
            assert!(
                app.network_sort_headers
                    .iter()
                    .any(|(_, index)| *index == sort.index())
            );
            let process = app
                .network_sort_headers
                .iter()
                .find(|(_, i)| *i == C::Process.index())
                .unwrap()
                .0;
            let pid = app
                .network_sort_headers
                .iter()
                .find(|(_, i)| *i == C::Pid.index())
                .unwrap()
                .0;
            assert!(process.right() <= pid.left());
        }
    }
}
