use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, View},
    config::{Config, Display},
    discovery::InventoryRates,
    model::*,
    ui,
};

fn network(name: &str, rate: f64, total: u64) -> Network {
    Network {
        name: name.into(),
        rx_per_sec: rate,
        tx_per_sec: rate / 2.,
        rx_total: total,
        tx_total: total / 2,
        ..Default::default()
    }
}
fn render(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn network_histories_are_scoped_and_session_totals_never_count_boot_traffic_or_resets() {
    let mut app = App::new(Config {
        history: 30,
        ..Default::default()
    });
    app.ingest(Snapshot {
        networks: vec![network("eth0", 20., 10_000), network("lo", 100., 100_000)],
        ..Default::default()
    });
    app.ingest(Snapshot {
        networks: vec![network("eth0", 40., 10_200), network("lo", 200., 101_000)],
        ..Default::default()
    });
    let h = app.history.network(&app.config).unwrap();
    assert_eq!(h.rx_session, 200);
    assert_eq!(h.rx_peak, 40.);
    app.config.display.network_interface = "lo".into();
    let h = app.history.network(&app.config).unwrap();
    assert_eq!(h.rx_session, 1000);
    assert_eq!(h.rx.iter().copied().collect::<Vec<_>>(), vec![100., 200.]);
    app.config.display.network_interface.clear();
    app.config.show_loopback = true;
    assert_eq!(app.history.network(&app.config).unwrap().rx_session, 1200);
    app.ingest(Snapshot {
        networks: vec![network("eth0", 0., 0), network("lo", 0., 0)],
        ..Default::default()
    });
    assert_eq!(app.history.network(&app.config).unwrap().rx_session, 1200);
    app.ingest(Snapshot {
        networks: vec![network("eth0", 2., 20), network("lo", 5., 50)],
        ..Default::default()
    });
    assert_eq!(app.history.network(&app.config).unwrap().rx_session, 1270);
    app.config.display.network_interface = "missing".into();
    assert!(app.history.network(&app.config).is_none());
    assert!(render(&mut app, 160, 48).contains("Selected interface unavailable"));
}
#[test]
fn container_rates_use_inventory_elapsed_time_identity_and_known_monotonic_counters() {
    let mut rates = InventoryRates::default();
    let mut inventory = Inventory {
        containers: vec![Container {
            id: "full-id".into(),
            runtime: "docker".into(),
            network_rx: Some(1000),
            network_tx: Some(300),
            block_read: Some(200),
            ..Default::default()
        }],
        ..Default::default()
    };
    rates.apply_at(&mut inventory, 0.);
    assert_eq!(inventory.containers[0].network_rx_per_sec, None);
    inventory.containers[0].network_rx = Some(1500);
    inventory.containers[0].network_tx = Some(400);
    inventory.containers[0].block_read = Some(400);
    rates.apply_at(&mut inventory, 5.);
    let c = &inventory.containers[0];
    assert_eq!(c.network_rx_per_sec, Some(100.));
    assert_eq!(c.network_tx_per_sec, Some(20.));
    assert_eq!(c.block_read_per_sec, Some(40.));
    assert_eq!(c.block_write_per_sec, None);
    inventory.containers[0].network_rx = Some(10);
    rates.apply_at(&mut inventory, 10.);
    assert_eq!(inventory.containers[0].network_rx_per_sec, None);
    inventory.containers[0].id = "replacement-id".into();
    rates.apply_at(&mut inventory, 15.);
    assert_eq!(inventory.containers[0].network_rx_per_sec, None);
    rates.apply_at(&mut inventory, 15.);
    assert_eq!(inventory.containers[0].network_rx_per_sec, None);
}
#[test]
fn program_group_totals_exclude_threads_and_expansion_keeps_real_process_identities() {
    let mut app = App::new(Config {
        show_threads: true,
        ..Default::default()
    });
    app.set_view(View::Processes);
    app.config.display.group_processes = true;
    app.ingest(Snapshot {
        processes: vec![
            Process {
                pid: 10,
                start_time: 1,
                name: "worker".into(),
                cpu: 125.,
                memory: 100,
                ..Default::default()
            },
            Process {
                pid: 11,
                start_time: 2,
                name: "worker".into(),
                cpu: 200.,
                memory: 200,
                ..Default::default()
            },
            Process {
                pid: 12,
                start_time: 2,
                name: "worker".into(),
                thread: true,
                cpu: 200.,
                memory: 200,
                ..Default::default()
            },
            Process {
                pid: 20,
                start_time: 3,
                name: "other".into(),
                cpu: 10.,
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    let groups = app.program_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].cpu, 325.);
    assert_eq!(groups[0].memory, 300);
    assert_eq!(groups[0].members.len(), 2);
    assert!(app.selected_process().is_none());
    assert!(render(&mut app, 120, 40).contains("PROGRAM GROUPS"));
    app.selected = 0;
    app.open_program_group();
    assert!(!app.process_grouped());
    assert_eq!(app.process_rows().len(), 2);
    app.snapshot.processes[0].start_time = 999;
    assert_eq!(
        app.process_rows().len(),
        1,
        "A reused PID must not join a previously expanded group"
    );
}
#[test]
fn units_preferences_and_all_new_settings_roundtrip_without_reinterpreting_old_layouts() {
    let mut config = Config::default();
    assert_eq!(14 + config.display.settings().len(), Config::SETTINGS_COUNT);
    for index in 0..config.display.settings().len() {
        config
            .display
            .adjust(index, true, &["eth0".into()], &["CPU package".into()]);
    }
    config.validate().unwrap();
    let saved = toml::to_string(&config).unwrap();
    let restored: Config = toml::from_str(&saved).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.layout, "balanced");
    assert_eq!(
        Display {
            temperature_unit: "fahrenheit".into(),
            ..Default::default()
        }
        .temperature(100., 0),
        "212°F"
    );
    assert_eq!(
        Display {
            temperature_unit: "kelvin".into(),
            ..Default::default()
        }
        .temperature(0., 2),
        "273.15 K"
    );
    assert_eq!(Display::default().rate(1024.), "1.0 KiB/s");
    assert_eq!(
        Display {
            network_bits: true,
            network_binary: false,
            ..Default::default()
        }
        .rate(125000.),
        "1.0 Mbit/s"
    );
    let mut invalid = restored;
    invalid.display.cpu_graph = "invented".into();
    assert!(invalid.validate().is_err());
}
#[test]
fn configured_metrics_expanded_panels_and_temperature_scales_render_from_recorded_data() {
    let mut app = App::new(Config::default());
    app.config.display.cpu_graph = "user".into();
    app.config.display.cpu_graph_lower = "system".into();
    app.config.display.temperature_unit = "fahrenheit".into();
    app.config.display.network_interface = "eth0".into();
    app.config.display.network_start_zero = true;
    app.config.display.network_bits = true;
    app.config.display.network_binary = false;
    app.config.display.network_logarithmic = true;
    for i in 0..40 {
        app.ingest(Snapshot {
            elapsed: i as f64,
            cpu_percent: 20.,
            cores: vec![20.; 32],
            cpu_breakdown: [("user".into(), 15.), ("system".into(), 5.)].into(),
            sensors: vec![Sensor {
                name: "CPU package".into(),
                temperature: 50.,
                critical: None,
            }],
            memory: Memory {
                total: 1000,
                used: 200,
                available: 800,
                cached: Some(300),
                free: Some(500),
                ..Default::default()
            },
            networks: vec![network("eth0", 125000., 1000000 + i * 1000)],
            ..Default::default()
        });
    }
    for (width, height) in [(120, 40), (160, 48), (240, 70)] {
        let text = render(&mut app, width, height);
        for marker in [
            "CPU history · user",
            "CPU history · system",
            "Temp 122.0°F",
            "Session",
            "Download history · log",
        ] {
            assert!(
                text.contains(marker),
                "Missing {marker} at {width}x{height}\n{text}"
            );
        }
    }
    app.config.display.overview_panel = "memory".into();
    app.config.display.memory_graphs = false;
    app.config.display.show_swap = false;
    let text = render(&mut app, 80, 24);
    assert!(text.contains("MEMORY") && text.contains("Cached") && !text.contains("Swap"));
    for panel in ["cpu", "network", "storage", "gpu", "all"] {
        app.config.display.overview_panel = panel.into();
        assert!(!render(&mut app, 60, 18).is_empty());
    }
}
#[test]
fn gpu_selection_respects_each_limit_and_never_treats_missing_usage_as_idle() {
    let mut devices = vec![
        Gpu {
            id: 0,
            uuid: "GPU-a".into(),
            vendor: "NVIDIA".into(),
            memory_total: Some(1000),
            memory_used: Some(100),
            utilization: Some(20),
            memory_utilization: Some(30),
            ..Default::default()
        },
        Gpu {
            id: 1,
            uuid: "GPU-b".into(),
            vendor: "NVIDIA".into(),
            memory_total: Some(2000),
            memory_used: Some(100),
            ..Default::default()
        },
    ];
    let mut limits = rstop::gpu::Selection {
        min_free_bytes: 800,
        max_gpu_utilization: Some(25),
        max_memory_utilization: Some(35),
        ..Default::default()
    };
    assert_eq!(rstop::gpu::select(&devices, &limits).len(), 1);
    limits.min_total_bytes = 1500;
    assert!(rstop::gpu::select(&devices, &limits).is_empty());
    devices[1].utilization = Some(0);
    devices[1].memory_utilization = Some(0);
    assert_eq!(rstop::gpu::select(&devices, &limits)[0].uuid, "GPU-b");
    devices[1]
        .extra
        .insert("mig_children".into(), serde_json::json!(2));
    assert!(rstop::gpu::select(&devices, &limits).is_empty());
}
#[test]
fn process_identity_fields_preserve_hex_capabilities_and_linux_stat_offsets() {
    let status = rstop::advanced::parse_process_fields(
        "CapEff:\t0000000000010000\nUid:\t1000 1001 1002 1003\nGid:\t10 11 12 13\nSeccomp:\t2\n",
    );
    assert_eq!(status["CapEff"], serde_json::json!("0000000000010000"));
    let mut fields = vec!["0"; 40];
    fields[5] = "123";
    fields[8] = "456";
    fields[10] = "789";
    fields[37] = "7";
    fields[38] = "2";
    let stat =
        rstop::advanced::parse_process_stat(&format!("42 (a ) tricky name) {}", fields.join(" ")));
    let process = Process {
        extra: [
            ("status".into(), serde_json::json!(status)),
            ("stat".into(), serde_json::json!(stat)),
        ]
        .into(),
        ..Default::default()
    };
    for (key, value) in [
        ("tpgid", 123),
        ("minor_faults_children", 456),
        ("major_faults_children", 789),
        ("rt_priority", 7),
        ("uid_effective", 1001),
        ("uid_saved", 1002),
        ("gid_effective", 11),
    ] {
        assert_eq!(
            rstop::fields::process_metric(&process, &Snapshot::default(), key),
            Some(serde_json::json!(value)),
            "{key}"
        );
    }
    assert_eq!(
        rstop::fields::process_display(&process, &Snapshot::default(), "scheduling"),
        "Round robin"
    );
    assert_eq!(
        rstop::fields::process_display(&process, &Snapshot::default(), "security"),
        "—"
    );
    assert!(rstop::process_tools::parse_io_priority("best-effort 7").is_ok());
    for invalid in [
        "idle 7",
        "realtime 8",
        "best-effort -1",
        "none 0 trailing",
        "1; touch /tmp/never",
    ] {
        assert!(rstop::process_tools::parse_io_priority(invalid).is_err());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn native_priority_controls_only_touch_an_owned_child_and_refuse_reused_identity() {
    use std::os::unix::process::CommandExt;
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut command = std::process::Command::new("sleep");
    command.arg("30");
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let child = Owned(command.spawn().unwrap());
    let pid = child.0.id();
    let mut system = sysinfo::System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
    );
    let start = system
        .process(sysinfo::Pid::from_u32(pid))
        .unwrap()
        .start_time();
    let mut target = Process {
        pid,
        start_time: start,
        name: "owned-priority-fixture".into(),
        ..Default::default()
    };
    rstop::process_tools::io_priority(&target, "idle").unwrap();
    assert_eq!(
        unsafe { libc::syscall(libc::SYS_ioprio_get, 1, pid) },
        3 << 13
    );
    target.start_time = start.saturating_add(1);
    assert!(rstop::process_tools::io_priority(&target, "none").is_err());
    assert_eq!(
        unsafe { libc::syscall(libc::SYS_ioprio_get, 1, pid) },
        3 << 13
    );
    assert!(rstop::process_tools::autogroup_priority(&target, 50).is_err());
}

#[test]
fn per_core_temperatures_use_the_selected_unit_and_never_replace_missing_core_values() {
    let mut app = App::new(Config::default());
    app.config.display.temperature_unit = "fahrenheit".into();
    app.ingest(Snapshot {
        cores: vec![20., 30., 40., 50.],
        core_temperatures: vec![Some(45.5), Some(60.), None, Some(80.)],
        ..Default::default()
    });
    let text = render(&mut app, 160, 48);
    let first = text.lines().find(|line| line.contains("C00")).unwrap();
    let second = text.lines().find(|line| line.contains("C01")).unwrap();
    assert!(
        first.contains("114°F") && first.contains("C02") && first.contains('—'),
        "{text}"
    );
    assert!(
        second.contains("140°F") && second.contains("176°F"),
        "{text}"
    );
}
