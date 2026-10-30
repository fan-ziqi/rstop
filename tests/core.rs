use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, Sort, View},
    collector::{core_times, parse_diskstats, parse_pressure, parse_proc_stat, rate},
    config::Config,
    discovery::{apply_container_stats, parse_containers, parse_listeners, parse_size},
    model::*,
    ui,
};

fn process(pid: u32, parent: Option<u32>, cpu: f32, name: &str) -> Process {
    Process {
        pid,
        parent,
        start_time: pid as u64,
        name: name.into(),
        command: format!("/usr/bin/{name}"),
        user: "tester".into(),
        cpu,
        ..Process::default()
    }
}

fn fixture() -> Snapshot {
    Snapshot {
        hostname: "test-host".into(),
        os: "test-os".into(),
        memory: Memory {
            total: 16 << 30,
            used: 4 << 30,
            available: 12 << 30,
            ..Memory::default()
        },
        cores: vec![10.0; 32],
        processes: vec![
            process(1, None, 0.0, "init"),
            process(2, Some(1), 50.0, "python"),
            process(3, Some(2), 25.0, "worker"),
        ],
        gpus: vec![Gpu {
            id: 0,
            name: "Test GPU".into(),
            vendor: "test".into(),
            utilization: Some(75),
            ..Gpu::default()
        }],
        inventory: Inventory {
            runtimes: vec!["docker".into()],
            service_managers: vec!["systemd/system".into()],
            containers: vec![Container {
                name: "test-container".into(),
                ..Container::default()
            }],
            services: vec![Service {
                name: "test.service".into(),
                ..Service::default()
            }],
            ..Inventory::default()
        },
        ..Snapshot::default()
    }
}

#[test]
fn tabs_use_visible_cells_and_compact_chrome_at_every_width() {
    for width in [60, 80, 120, 160] {
        let mut app = App::new(Config::default());
        app.ingest(fixture());
        app.status.clear();
        let mut terminal = Terminal::new(TestBackend::new(width, 32)).unwrap();
        for view in app.views() {
            app.set_view(view);
            terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
            assert!(app.tab_ranges.iter().any(|(_, v)| *v == view));
            let buffer = terminal.backend().buffer();
            let mut last = 0;
            for (rect, tab) in &app.tab_ranges {
                assert_eq!(rect.y, 1);
                assert_eq!(rect.height, 3);
                assert!(rect.x >= last);
                assert_eq!(buffer[(rect.x, rect.y)].symbol(), "╭");
                assert_eq!(buffer[(rect.right() - 1, rect.y)].symbol(), "╮");
                if *tab != view {
                    assert_eq!(
                        buffer[(rect.x + 1, rect.y + 1)].bg,
                        ui::palette(&app.config.theme).bg
                    );
                }
                last = rect.right() + 1;
            }
            assert_eq!(app.filter_rect.height, u16::from(app.can_filter()));
            assert!(
                app.footer_actions
                    .iter()
                    .all(|(rect, _)| rect.y >= 30 && rect.bottom() <= 32)
            );
            assert_eq!(app.footer_actions.last().unwrap().0.bottom(), 32);
        }
        app.set_view(View::Processes);
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        let before = app.table_rect;
        let filter_before = app.filter_rect;
        app.editing_filter = true;
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        assert_eq!(app.table_rect, before);
        assert_eq!(app.filter_rect, filter_before);
        assert_eq!(app.filter_rect.height, 1);
    }
}

#[test]
fn rates_handle_counter_reset_new_devices_and_zero_time() {
    assert_eq!(rate(Some(100), 300, 2.0), 100.0);
    assert_eq!(rate(Some(300), 100, 2.0), 0.0);
    assert_eq!(rate(None, 900000, 1.0), 0.0);
    assert_eq!(rate(Some(0), 100, 0.0), 0.0);
}

#[test]
fn cpu_state_deltas_normalize_without_double_counting_guest() {
    let states =
        rstop::collector::cpu_breakdown(&[100; 8], &[120, 100, 110, 150, 110, 100, 100, 110]);
    assert_eq!(states["user"], 20.0);
    assert_eq!(states["iowait"], 10.0);
    assert_eq!(states.values().sum::<f64>(), 100.0);
    assert!(rstop::collector::cpu_breakdown(&[100; 8], &[0; 8]).is_empty());
}

#[test]
fn local_only_container_endpoints_reject_remote_contexts() {
    use rstop::discovery::is_local_endpoint;
    for endpoint in [
        "unix:///var/run/docker.sock",
        "npipe:////./pipe/docker_engine",
        "tcp://127.0.0.1:2375",
        "tcp://localhost",
        "tcp://[::1]:2375",
    ] {
        assert!(is_local_endpoint(endpoint));
    }
    for endpoint in [
        "ssh://host",
        "tcp://192.168.1.20:2375",
        "tcp://localhost.example.com:2375",
        "tcp://127.0.0.10:2375",
        "npipe:////remote-server/pipe/docker_engine",
    ] {
        assert!(!is_local_endpoint(endpoint));
    }
}

#[test]
fn proc_stat_parses_aggregate_and_present_cores_in_file_order() {
    let text = "\
cpu  100 20 30 400 50 6 7 8 9 10
cpu0 40 5 10 100 12 1 2 3 0 0
cpu2 30 10 10 200 25 3 3 2 0 0
intr 1 2 3
ctxt 98765
btime 1234
processes 567
procs_running 2
procs_blocked 0
";
    let (aggregate, cores) = parse_proc_stat(text);
    assert_eq!(aggregate, vec![100, 20, 30, 400, 50, 6, 7, 8, 9, 10]);
    assert_eq!(
        cores,
        vec![
            vec![40, 5, 10, 100, 12, 1, 2, 3, 0, 0],
            vec![30, 10, 10, 200, 25, 3, 3, 2, 0, 0],
        ]
    );
}

#[test]
fn proc_stat_ignores_malformed_short_and_non_cpu_lines() {
    let text = "\
cpux 1 2 3 4 5 6 7 8
cpu0 1 2 3
cpu1 1 2 three 4 5 6 7 8
cpu2 1 2 3 4 5 6 7 8
cpu3 1 2 3 4 5 6 7 8 extra
cpu4 1 2 3 4 5 6 7 8 9 10
cpu  1 2 3 4 5 6 7 8 9 10
";
    let (aggregate, cores) = parse_proc_stat(text);
    assert_eq!(aggregate, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(
        cores,
        vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ]
    );
    assert_eq!(parse_proc_stat("intr 1\nctxt 2\n"), (vec![], vec![]));
}

#[test]
fn per_core_cpu_times_diff_matching_samples_and_reject_misalignment() {
    let previous = "\
cpu  0 0 0 0 0 0 0 0
cpu0 100 0 100 500 0 0 0 0
cpu1 100 0 100 500 0 0 0 0
";
    let current = "\
cpu  0 0 0 0 0 0 0 0
cpu0 150 0 120 510 20 0 0 0
cpu1 125 0 125 550 0 0 0 0
";
    let (_, previous) = parse_proc_stat(previous);
    let (_, current) = parse_proc_stat(current);
    assert_eq!(
        core_times(&previous, &current, 2),
        vec![
            CoreTime {
                user: 50.,
                nice: 0.,
                system: 20.,
                iowait: 20.,
                steal: 0.,
            },
            CoreTime {
                user: 25.,
                nice: 0.,
                system: 25.,
                iowait: 0.,
                steal: 0.,
            },
        ]
    );
    assert!(core_times(&[], &current, 2).is_empty());
    assert!(core_times(&previous, &current, 4).is_empty());
    let mut hotplugged = previous.clone();
    hotplugged.push(vec![0; 8]);
    assert!(core_times(&hotplugged, &current, 2).is_empty());
}

#[test]
fn core_time_groups_user_nice_system_and_iowait_shares() {
    assert_eq!(CoreTime::from_breakdown(&Default::default()), None);
    let breakdown: std::collections::BTreeMap<String, f64> = [
        ("user", 10.),
        ("nice", 5.),
        ("system", 20.),
        ("irq", 3.),
        ("softirq", 2.),
        ("iowait", 7.),
        ("steal", 1.),
        ("idle", 52.),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value))
    .collect();
    assert_eq!(
        CoreTime::from_breakdown(&breakdown),
        Some(CoreTime {
            user: 10.,
            nice: 5.,
            system: 25.,
            iowait: 7.,
            steal: 1.,
        })
    );
}

#[test]
fn diskstats_uses_512_byte_sectors_and_correct_busy_field() {
    let counters = parse_diskstats(
        "259 0 nvme0n1 1 2 100 4 5 6 200 8 9 123 11\n7 0 loop0 1 2 100 4 5 6 200 8 9 123 11\ninvalid",
    );
    assert_eq!(counters.len(), 1);
    assert_eq!(counters["nvme0n1"], (51200, 102400, 123));
}

#[test]
fn psi_missing_full_is_unknown_not_zero() {
    let pressure = parse_pressure("cpu", "some avg10=1.23 avg60=2.34 avg300=3.45 total=12\n");
    assert_eq!(pressure.some_10, Some(1.23));
    assert_eq!(pressure.some_60, Some(2.34));
    assert_eq!(pressure.full_10, None);
}

#[test]
fn container_units_preserve_decimal_and_binary_semantics() {
    assert_eq!(parse_size(" 1.5 GiB "), Some(1610612736));
    assert_eq!(parse_size("1.5GB"), Some(1500000000));
    assert_eq!(parse_size("0B"), Some(0));
    assert_eq!(parse_size("—"), None);
    assert_eq!(parse_size("NaN GiB"), None);
    assert_eq!(parse_size("-2MB"), None);
}

#[test]
fn docker_stats_match_truncated_ids_and_keep_missing_stats_unknown() {
    let mut containers = parse_containers(
        "docker",
        r#"{"ID":"abcdef1234567890","Names":"train","State":"running","Image":"cuda"}
{"ID":"bcd1234567890","Names":"stopped","State":"exited"}
not json"#,
    );
    apply_container_stats(
        &mut containers,
        r#"{"ID":"abcdef123456","CPUPerc":"205.40%","MemUsage":"1.5GiB / 8GiB","NetIO":"4.1MB / 2kB","BlockIO":"4MiB / 7kB","MemPerc":"18.75%","PIDs":"8"}"#,
    );
    assert_eq!(containers.len(), 2);
    assert_eq!(containers[0].cpu, Some(205.4));
    assert_eq!(containers[0].memory, Some(1610612736));
    assert_eq!(containers[0].network_rx, Some(4100000));
    assert_eq!(containers[0].block_read, Some(4 * 1024 * 1024));
    assert_eq!(containers[0].block_write, Some(7000));
    assert_eq!(containers[0].memory_percent, Some(18.75));
    assert_eq!(containers[1].memory, None);
}

#[test]
fn ipv4_ipv6_listener_parser_excludes_established_tcp() {
    let tcp = "header\n0: 0100007F:1F90 00000000:0000 0A 0 0 0 1000 0\n1: 0100007F:0016 00000000:0000 01 0 0 0 1000 0";
    let listeners = parse_listeners(tcp, "TCP");
    assert_eq!(listeners.len(), 1);
    assert_eq!(listeners[0].address, "127.0.0.1");
    assert_eq!(listeners[0].port, 8080);
    let tcp6 = "header\n0: 00000000000000000000000001000000:01BB 00000000:0000 0A 0 0 0 1000 0";
    let listeners = parse_listeners(tcp6, "TCP6");
    assert_eq!(listeners[0].address, "::1");
    assert_eq!(listeners[0].port, 443);
}

#[test]
fn all_resource_views_remain_available_before_and_after_discovery() {
    let mut app = App::new(Config::default());
    let views = vec![
        View::Overview,
        View::Processes,
        View::Gpu,
        View::Storage,
        View::Network,
        View::System,
        View::Containers,
        View::Services,
    ];
    assert_eq!(app.views(), views);
    let mut terminal = Terminal::new(TestBackend::new(140, 44)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_eq!(
        app.tab_ranges
            .iter()
            .map(|(_, view)| *view)
            .collect::<Vec<_>>(),
        views
    );
    for view in [View::Gpu, View::Containers, View::Services] {
        app.set_view(view);
        app.ingest(fixture());
        assert_eq!(app.views(), views);
        assert_eq!(app.view, view);
        app.ingest(Snapshot::default());
        assert_eq!(app.views(), views);
        assert_eq!(app.view, view);
    }
}

#[test]
fn selection_tracks_process_identity_across_resort_and_pid_reuse() {
    let mut app = App::new(Config::default());
    app.set_view(View::Processes);
    let mut snapshot = fixture();
    app.ingest(snapshot.clone());
    app.selected = 0;
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 2);
    snapshot.processes[2].cpu = 100.0;
    app.ingest(snapshot.clone());
    assert_eq!(app.selected_process().unwrap().pid, 2);
    assert_eq!(app.selected, 0); // Live values update while the selected order is held.
    snapshot.processes[1].start_time = 999;
    app.ingest(snapshot);
    assert_eq!(app.selected_process, None);
}

#[test]
fn filtered_tree_retains_ancestors_and_collapses_descendants() {
    let mut app = App::new(Config::default());
    app.ingest(fixture());
    app.tree = true;
    app.filter = "worker".into();
    let rows = app.process_rows();
    assert_eq!(
        rows.iter()
            .map(|(i, d)| (app.snapshot.processes[*i].pid, *d))
            .collect::<Vec<_>>(),
        vec![(1, 0), (2, 1), (3, 2)]
    );
    app.collapsed.insert(2);
    assert_eq!(app.process_rows().len(), 2);
}

#[test]
fn malformed_parent_cycles_terminate_without_duplicate_rows() {
    let mut app = App::new(Config::default());
    let snapshot = Snapshot {
        processes: vec![process(1, Some(2), 0.0, "a"), process(2, Some(1), 1.0, "b")],
        ..Snapshot::default()
    };
    app.ingest(snapshot);
    app.tree = true;
    app.filter = "a".into();
    let rows = app.process_rows();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].0, rows[1].0);
}

#[test]
fn history_is_bounded_and_accepts_each_new_sample() {
    let mut app = App::new(Config {
        history: 30,
        ..Config::default()
    });
    for i in 0..100 {
        let mut snapshot = fixture();
        snapshot.timestamp = i;
        app.ingest(snapshot);
    }
    assert_eq!(app.history.cpu.len(), 30);
    app.ingest(Snapshot {
        timestamp: 100,
        ..Default::default()
    });
    assert_eq!(app.snapshot.timestamp, 100);
    assert_eq!(app.history.cpu.len(), 30);
}

#[test]
fn sorting_is_deterministic_and_multi_core_cpu_is_preserved() {
    let mut app = App::new(Config::default());
    let mut snapshot = fixture();
    snapshot.processes[1].cpu = 400.0;
    app.ingest(snapshot);
    assert_eq!(app.snapshot.processes[app.process_rows()[0].0].cpu, 400.0);
    app.sort = Sort::Pid;
    app.reverse = true;
    assert_eq!(app.snapshot.processes[app.process_rows()[0].0].pid, 3);
}

#[test]
fn gpu_filter_is_scoped_to_selected_device() {
    let mut app = App::new(Config::default());
    let mut snapshot = fixture();
    snapshot.processes[1].gpu_ids = vec![0];
    snapshot.processes[2].gpu_ids = vec![1];
    app.ingest(snapshot);
    app.view = View::Gpu;
    assert_eq!(app.process_rows().len(), 1);
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 2);
}

#[test]
fn config_rejects_invalid_values_and_typos() {
    assert!(
        Config {
            interval_ms: 0,
            ..Config::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Config {
            history: usize::MAX,
            ..Config::default()
        }
        .validate()
        .is_err()
    );
    assert!(toml::from_str::<Config>("intervall_ms = 500").is_err());
    assert!(
        Config {
            theme: "typo".into(),
            ..Config::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn every_view_and_overlay_renders_at_all_supported_sizes() {
    for &theme in rstop::config::THEMES {
        for (width, height) in [
            (1, 1),
            (59, 17),
            (60, 18),
            (80, 24),
            (100, 30),
            (160, 48),
            (240, 70),
        ] {
            let mut app = App::new(Config {
                theme: theme.into(),
                ..Config::default()
            });
            app.ingest(fixture());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for view in app.views() {
                app.view = view;
                terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
            }
            app.help = true;
            terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
            app.help = false;
            app.settings = true;
            terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
            app.settings = false;
            app.detail = Some(("Details".into(), "a\nline\nwrapped\n".repeat(30)));
            terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        }
    }
}

#[test]
fn resource_failures_are_reported_in_host_alerts() {
    let mut snapshot = Snapshot::default();
    snapshot.apply_inventory(Inventory {
        services: vec![Service {
            name: "failed.service".into(),
            active: "failed".into(),
            ..Service::default()
        }],
        containers: vec![Container {
            name: "restarting-container".into(),
            state: "restarting".into(),
            status: "Restarting".into(),
            ..Container::default()
        }],
        ..Inventory::default()
    });
    assert_eq!(snapshot.alerts.len(), 2);
}

#[test]
fn memory_histories_preserve_all_four_metrics_and_missing_recorded_values() {
    let old: Memory = serde_json::from_str(r#"{"total":1000,"used":200,"available":800}"#).unwrap();
    assert_eq!(old.free, None);
    assert_eq!(old.cached, None);
    let mut app = App::new(Config {
        history: 2,
        ..Default::default()
    });
    let mut snapshot = Snapshot {
        memory: old,
        ..Default::default()
    };
    app.ingest(snapshot.clone());
    assert!(app.history.series["RAM free (%)"][0].is_nan());
    assert!(app.history.series["RAM cached (%)"][0].is_nan());
    snapshot.memory.free = Some(500);
    snapshot.memory.cached = Some(300);
    app.ingest(snapshot.clone());
    assert_eq!(app.history.memory[1], 20.);
    assert_eq!(app.history.series["RAM available (%)"][1], 80.);
    assert_eq!(app.history.series["RAM cached (%)"][1], 30.);
    assert_eq!(app.history.series["RAM free (%)"][1], 50.);
    snapshot.memory.free = Some(0);
    app.ingest(snapshot);
    assert_eq!(app.history.series["RAM free (%)"].len(), 2);
    assert_eq!(app.history.series["RAM free (%)"][1], 0.);
}

#[test]
fn network_top_is_a_session_peak_of_simultaneous_traffic_in_each_scope() {
    let mut app = App::new(Config {
        history: 2,
        ..Default::default()
    });
    let mut snapshot = Snapshot {
        networks: vec![
            Network {
                name: "eth0".into(),
                rx_per_sec: 100.,
                tx_per_sec: 10.,
                ..Default::default()
            },
            Network {
                name: "eth1".into(),
                rx_per_sec: 0.,
                tx_per_sec: 20.,
                ..Default::default()
            },
            Network {
                name: "lo".into(),
                rx_per_sec: 900.,
                tx_per_sec: 900.,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    app.ingest(snapshot.clone());
    snapshot.networks[0].rx_per_sec = 0.;
    snapshot.networks[1].rx_per_sec = 80.;
    snapshot.networks[2].rx_per_sec = 0.;
    snapshot.networks[2].tx_per_sec = 0.;
    for _ in 0..5 {
        app.ingest(snapshot.clone());
    }
    assert_eq!(app.history.rx.len(), 2);
    assert_eq!(app.history.rx[0], 80.);
    assert_eq!(app.history.network_peaks[0].rx, 100.);
    assert_eq!(app.history.network_peaks[0].tx, 30.);
    assert_eq!(app.history.network_peaks[1].rx, 1000.);
    assert_eq!(app.history.network_peaks[1].tx, 930.);
    app.config.show_loopback = true;
    app.ingest(snapshot);
    assert_eq!(app.history.network_peaks[1].rx, 1000.);
    app.ingest(Snapshot {
        networks: vec![Network {
            name: "eth0".into(),
            rx_per_sec: 1e9,
            ..Default::default()
        }],
        ..Default::default()
    });
    assert_eq!(app.history.network_peaks[0].rx, 1e9);
}

#[test]
fn legacy_layout_names_and_nested_layouts_keep_their_configuration_contract() {
    for layout in ["balanced", "compute", "storage", "minimal"] {
        let config: Config = toml::from_str(&format!("layout = {layout:?}\n")).unwrap();
        config.validate().unwrap();
        let roundtrip: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(roundtrip.layout, layout);
    }
    let config: Config = toml::from_str(r#"layout = "minimal"
custom_layout = { direction = "vertical", children = [{panel="cpu"}, {direction="horizontal",children=[{panel="memory"},{panel="network"}]}] }
"#).unwrap();
    config.validate().unwrap();
    assert_eq!(config.layout, "minimal");
    assert!(config.custom_layout.is_some());
}
