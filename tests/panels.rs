use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, View},
    config::Config,
    connections::{Connection, Sample},
    model::{Gpu, GpuProcess, Inventory, Network, Process, Snapshot},
    network_sort::{ConnectionSort, InterfaceSort},
    ui,
};

fn fixture() -> Snapshot {
    Snapshot {
        timestamp: 100,
        processes: vec![Process {
            pid: 42,
            start_time: 10,
            name: "worker".into(),
            command: "worker".into(),
            gpu_ids: vec![0],
            ..Default::default()
        }],
        networks: (0..20)
            .map(|i| Network {
                name: format!("eth{i}"),
                rx_per_sec: i as f64,
                ..Default::default()
            })
            .collect(),
        connections: Sample {
            sampled_at: 100,
            connections: (0..40)
                .map(|i| Connection {
                    id: format!("socket{i}"),
                    process_names: vec!["worker".into()],
                    pids: vec![42],
                    local: format!("127.0.0.1:{}", 3000 + i),
                    remote: "192.0.2.1:443".into(),
                    rx_per_sec: Some(i as f64),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        gpus: vec![Gpu {
            id: 0,
            uuid: "test".into(),
            name: "Test GPU".into(),
            vendor: "NVIDIA".into(),
            driver: "595.1".into(),
            utilization: Some(70),
            memory_total: Some(1024),
            memory_used: Some(512),
            processes: vec![GpuProcess {
                pid: 42,
                ..Default::default()
            }],
            extra: [("pci_bus_id".into(), serde_json::json!("0000:01:00.0"))].into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn draw(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn unsampled_gpu_process_count_is_unknown_rather_than_zero() {
    let mut app = App::new(Config::default());
    let mut data = fixture();
    data.gpus[0].processes.clear();
    data.extra.insert(
        "Collection state".into(),
        serde_json::json!({"gpu_processes":false}),
    );
    app.ingest(data.clone());
    assert!(draw(&mut app, 160, 48).contains("— processes"));
    data.extra.insert(
        "Collection state".into(),
        serde_json::json!({"gpu_processes":true}),
    );
    app.ingest(data);
    assert!(draw(&mut app, 160, 48).contains("0 processes"));
}

#[test]
fn empty_resource_pages_distinguish_loading_absent_and_connected_states() {
    for (width, height) in [(60, 18), (140, 44)] {
        for (view, loading, absent, connected) in [
            (
                View::Gpu,
                "Loading GPU telemetry",
                "No readable GPU telemetry",
                "No readable GPU telemetry",
            ),
            (
                View::Containers,
                "Loading containers",
                "No readable container engines",
                "No containers",
            ),
            (
                View::Services,
                "Loading services",
                "No readable service manager",
                "No services",
            ),
        ] {
            let mut app = App::new(Config::default());
            app.set_view(view);
            assert!(draw(&mut app, width, height).contains(loading));
            app.ingest(Snapshot {
                timestamp: 100,
                gpu_status: "Driver unavailable".into(),
                ..Default::default()
            });
            let text = draw(&mut app, width, height);
            assert!(text.contains(if view == View::Gpu { absent } else { loading }));
            app.ingest(Snapshot {
                timestamp: 101,
                gpu_status: "Driver unavailable".into(),
                inventory: Inventory {
                    updated_at: 101,
                    ..Default::default()
                },
                ..Default::default()
            });
            let text = draw(&mut app, width, height);
            assert!(text.contains(absent), "{view:?} {width}x{height}: {text}");
            assert!(!text.contains(loading));
            if view == View::Gpu {
                assert!(text.contains("Driver unavailable"));
            }
            app.snapshot.inventory.runtimes.push("docker".into());
            app.snapshot
                .inventory
                .service_managers
                .push("systemd".into());
            assert!(draw(&mut app, width, height).contains(connected));
        }
    }
}

#[test]
fn disabled_gpu_and_recorded_empty_pages_do_not_show_live_loading_messages() {
    let mut app = App::new(Config {
        collect_gpu: false,
        ..Default::default()
    });
    app.set_view(View::Gpu);
    assert!(draw(&mut app, 140, 44).contains("GPU collection disabled"));
    app.replay = true;
    for (view, empty) in [
        (View::Gpu, "No readable GPU telemetry"),
        (View::Containers, "No readable container engines"),
        (View::Services, "No readable service manager"),
    ] {
        app.set_view(view);
        let text = draw(&mut app, 140, 44);
        assert!(text.contains(empty));
        assert!(!text.contains("Loading"));
        assert!(!text.contains("Discovering local"));
    }
}

#[test]
fn network_and_gpu_tables_stay_visible_together_at_all_supported_sizes() {
    for (width, height) in [(60, 18), (80, 24), (120, 36), (160, 48)] {
        let mut app = App::new(Config::default());
        app.ingest(fixture());
        for (view, focus, labels) in [
            (View::Network, 1, ["CONNECTIONS", "INTERFACES"]),
            (View::Gpu, 2, ["PROCESSES", "DEVICE INFO"]),
        ] {
            app.set_view(view);
            for section in [0, focus] {
                app.focus_panel(section);
                let text = draw(&mut app, width, height);
                for label in labels {
                    assert!(
                        text.contains(label),
                        "{view:?} {width}x{height}: {label}\n{text}"
                    );
                }
                assert_eq!(app.panel_regions.len(), 2);
                for region in &app.panel_regions {
                    assert!(
                        region.table.height >= 1,
                        "{view:?} {width}x{height}: no data rows"
                    );
                    assert!(region.table.right() <= width && region.table.bottom() < height);
                }
                assert_eq!(app.focused_panel(), section);
                if section == 0 {
                    app.move_selection(1);
                    let selected_text = draw(&mut app, width, height);
                    for label in labels {
                        assert!(selected_text.contains(label));
                    }
                    for region in &app.panel_regions {
                        assert!(
                            region.table.height >= 1,
                            "Selected {view:?} {width}x{height}: no data rows"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn panel_filters_selection_offsets_and_sort_directions_survive_focusing_and_redrawing() {
    let mut app = App::new(Config::default());
    app.ingest(fixture());
    app.set_view(View::Network);
    app.filter = "port:443".into();
    app.selected = 24;
    app.table_offset = 20;
    app.remember_selection();
    let selected = app.selected_connection.clone();
    app.set_network_sort(ConnectionSort::Pid.index(), true);
    app.focus_panel(1);
    app.filter = "eth1".into();
    app.selected = 5;
    app.table_offset = 4;
    app.remember_selection();
    let interface = app.selected_interface.clone();
    app.set_network_sort(InterfaceSort::Name.index(), true);
    for _ in 0..3 {
        draw(&mut app, 160, 48);
        assert_eq!(app.network_section, 1);
        assert_eq!(app.filter, "eth1");
        app.focus_panel(0);
        assert_eq!(app.filter, "port:443");
        assert_eq!(app.selected_connection, selected);
        assert_eq!(
            app.filtered_connections()[app.selected].id,
            selected.clone().unwrap()
        );
        app.focus_panel(1);
        assert_eq!(app.selected_interface, interface);
    }
    app.focus_panel(0);
    app.begin_filter();
    app.filter = "process:worker".into();
    app.editor.cursor = Some(3);
    draw(&mut app, 80, 24);
    assert!(app.editing_filter);
    assert_eq!(app.filter_before_edit, "port:443");
    assert_eq!(app.editor.cursor, Some(3));
}

#[test]
fn gpu_metadata_filter_does_not_clear_or_reassign_a_selected_process() {
    let mut app = App::new(Config::default());
    app.ingest(fixture());
    app.set_view(View::Gpu);
    app.filter = "worker".into();
    app.move_selection(1);
    assert_eq!(app.selected_process, Some((42, 10)));
    app.focus_panel(2);
    app.begin_filter();
    app.filter = "pci".into();
    app.editing_filter = false;
    app.remember_selection();
    draw(&mut app, 160, 48);
    app.ingest(fixture());
    assert_eq!(app.gpu_section, 2);
    assert_eq!(app.filter, "pci");
    assert_eq!(app.selected_process, Some((42, 10)));
    app.focus_panel(0);
    assert_eq!(app.filter, "worker");
    app.focus_panel(2);
    let mut reused = fixture();
    reused.processes[0].start_time = 20;
    app.ingest(reused);
    assert!(app.selected_process.is_none());
    assert_eq!(app.filter, "pci");
}
