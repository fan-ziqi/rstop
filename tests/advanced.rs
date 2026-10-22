use ratatui::{Terminal, backend::TestBackend, style::Color};
use rstop::{
    advanced::*,
    app::{App, View},
    config::Config,
    events::Events,
    filter::Query,
    model::*,
    recording::{Recorder, csv, replay},
};
use std::{fs, path::PathBuf, time::Duration};
struct Temp(PathBuf);
impl Temp {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "rstop-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn proc() -> Process {
    Process {
        pid: 42,
        start_time: 100,
        name: "Python Worker".into(),
        command: "/bin/python train.py".into(),
        user: "alice".into(),
        cpu: 150.,
        memory: 2 << 30,
        ..Default::default()
    }
}
#[test]
fn expression_filter_regex_numeric_and_or_negation_quotes() {
    let p = proc();
    for q in [
        "user:alice cpu:>100",
        "name:\"python worker\" !user:bob",
        "name:/^python/",
        "pid:999 | mem:>1024",
        "!gpu:>0",
        "",
    ] {
        assert!(Query::parse(q).matches(&p), "{q}");
    }
    for q in [
        "user:bob cpu:>100",
        "cpu:<100",
        "vram:=0",
        "gpu:=0",
        "pid:999 | name:rust",
    ] {
        assert!(!Query::parse(q).matches(&p), "{q}");
    }
    for q in ["name:/[/", "typo:foo", "name:\"unclosed"] {
        assert!(Query::parse(q).error.is_some(), "{q}");
        assert!(!Query::parse(q).matches(&p));
    }
}
#[test]
fn alert_hysteresis_uses_stable_keys_and_records_recovery() {
    let mut events = Events::default();
    assert!(events.update(100, &["CPU 91%".into()], 3, 3).is_empty());
    assert!(events.update(102, &["CPU 94%".into()], 3, 3).is_empty());
    let active = events.update(103, &["CPU 97%".into()], 3, 3);
    assert_eq!(active.len(), 1);
    assert!(active[0].active);
    assert!(events.update(104, &["CPU 99%".into()], 3, 3).is_empty());
    let clear = events.update(105, &[], 3, 3);
    assert_eq!(clear.len(), 1);
    assert!(!clear[0].active);
    events.update(106, &["Memory 95%".into()], 0, 3);
    events.update(107, &[], 0, 3);
    assert_eq!(events.history.len(), 3);
}
#[test]
fn transient_alert_is_discarded_before_hold() {
    let mut events = Events::default();
    events.update(1, &["CPU 99%".into()], 3, 10);
    events.update(2, &[], 3, 10);
    events.update(4, &["CPU 97%".into()], 3, 10);
    assert!(events.history.is_empty());
}
#[test]
fn local_api_validation_rejects_spoofed_authorities() {
    for url in [
        "https://127.0.0.1:6443",
        "http://localhost:8080/api",
        "https://[::1]:6443",
        "http://localhost",
    ] {
        assert!(local_http(url), "{url}");
    }
    for url in [
        "https://localhost.evil:443",
        "https://127.0.0.1:443@evil",
        "https://[::1]evil:443",
        "https://10.0.0.1:6443",
        "http://localhost\\@evil",
        "ssh://localhost",
        "https://localhost:\n443",
    ] {
        assert!(!local_http(url), "{url}");
    }
}
#[test]
fn fdinfo_and_full_diskstats_preserve_units_and_field_positions() {
    let fields = parse_fdinfo(
        "drm-pdev:\t0000:01:00.0\ndrm-client-id: 17\ndrm-engine-render: 3000000 ns\ndrm-memory-vram: 10 KiB\ninvalid",
    );
    assert_eq!(fields["drm-pdev"], "0000:01:00.0");
    assert_eq!(fields["drm-engine-render"], "3000000 ns");
    let disks =
        parse_full_diskstats("259 0 nvme0n1 100 0 300 400 500 0 700 800 9 1000 1100\ninvalid");
    assert_eq!(disks["nvme0n1"][0], 100);
    assert_eq!(disks["nvme0n1"][4], 500);
    assert_eq!(disks["nvme0n1"][10], 1100);
}
#[test]
fn connection_addresses_decode_ipv4_and_ipv6() {
    assert_eq!(
        rstop::connections::decode_address("0100007F:1F90"),
        "127.0.0.1:8080"
    );
    assert_eq!(
        rstop::connections::decode_address("00000000000000000000000001000000:01BB"),
        "[::1]:443"
    );
    assert_eq!(
        rstop::connections::decode_address("not-an-address"),
        "not-an-address"
    );
}
#[test]
fn folder_scan_reports_bounds_and_does_not_follow_symlinks() {
    let t = Temp::new("folders");
    fs::write(t.0.join("data"), b"12345").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&t.0, t.0.join("loop")).unwrap();
    let value = folder_size(&t.0, 100, Duration::from_secs(1));
    assert_eq!(value["bytes"], 5);
    assert_eq!(value["complete"], true);
    assert_eq!(
        folder_size(&t.0, 1, Duration::from_secs(1))["complete"],
        false
    );
    assert_eq!(
        folder_size(&t.0.join("missing"), 100, Duration::from_secs(1))["errors"],
        1
    );
}
#[test]
fn recording_round_trip_rotation_and_legacy_defaults() {
    let t = Temp::new("record");
    let path = t.0.join("samples.jsonl");
    let mut recorder = Recorder::new(&path, 1).unwrap();
    let mut sample = Snapshot {
        timestamp: 123,
        processes: vec![proc()],
        ..Default::default()
    };
    sample
        .extra
        .insert("large".into(), serde_json::json!("x".repeat(600_000)));
    for time in 1..=5 {
        sample.timestamp = time;
        recorder.write(&sample).unwrap();
    }
    drop(recorder);
    assert_eq!(replay(&path).unwrap()[0].timestamp, 5);
    assert_eq!(
        replay(&t.0.join("samples.jsonl.1")).unwrap()[0].timestamp,
        4
    );
    assert_eq!(
        replay(&t.0.join("samples.jsonl.3")).unwrap()[0].timestamp,
        2
    );
    fs::write(&path, "{\"timestamp\":9}\n").unwrap();
    assert_eq!(replay(&path).unwrap()[0].timestamp, 9);
    fs::write(&path, "{\"timestamp\":9}\ntruncated").unwrap();
    assert!(replay(&path).unwrap_err().to_string().contains("line 2"));
    fs::write(&path, "").unwrap();
    assert!(replay(&path).is_err());
}
#[test]
fn csv_escapes_quotes_newlines_and_nulls() {
    let mut snapshot = Snapshot {
        timestamp: 5,
        hostname: "a,\"b\"\nc".into(),
        ..Default::default()
    };
    snapshot
        .extra
        .insert("unknown".into(), serde_json::Value::Null);
    let mut bytes = vec![];
    csv(&snapshot, &mut bytes, true).unwrap();
    let output = String::from_utf8(bytes).unwrap();
    assert!(output.starts_with("timestamp,path,value\n"));
    assert!(output.contains("5,\"system.hostname\",\"a,\"\"b\"\"\nc\""));
    assert!(output.contains("\"system.extra.unknown\",\"null\""));
}
#[test]
fn cpu_masks_reject_empty_descending_and_huge_ranges() {
    assert_eq!(
        rstop::process_tools::cpu_list("0,2-4,3").unwrap(),
        vec![0, 2, 3, 4]
    );
    for text in ["", "1,", "3-1", "0-999999999", "-1"] {
        assert!(rstop::process_tools::cpu_list(text).is_err(), "{text}");
    }
}
#[test]
fn new_configuration_values_validate_and_round_trip() {
    let t = Temp::new("config");
    let path = t.0.join("settings.toml");
    let mut c = Config {
        layout: "compute".into(),
        columns: vec!["pid".into(), "cpu".into(), "command".into()],
        ..Default::default()
    };
    c.custom_colors.insert("accent".into(), "#80f0ba".into());
    c.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap().columns, c.columns);
    c.custom_colors.insert("text".into(), "red".into());
    assert!(c.validate().is_err());
    c.custom_colors.remove("text");
    c.alert_commands = vec![vec![]];
    assert!(c.validate().is_err());
    c.alert_commands.clear();
    c.alert_hold_seconds = 3601;
    assert!(c.validate().is_err());
}
#[test]
fn all_layouts_new_pages_and_ascii_no_color_render() {
    for layout in ["balanced", "compute", "storage", "minimal"] {
        let mut app = App::new(Config {
            layout: layout.into(),
            ascii: true,
            no_color: true,
            ..Default::default()
        });
        let mut snapshot = Snapshot {
            processes: vec![proc()],
            cores: vec![20.; 32],
            ..Default::default()
        };
        snapshot
            .extra
            .insert("sensor".into(), serde_json::json!({"watts":42}));
        app.ingest(snapshot);
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        for view in [View::Overview, View::Explorer, View::History, View::Events] {
            app.set_view(view);
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            for cell in terminal.backend().buffer().content() {
                assert!(cell.symbol().is_ascii(), "{}", cell.symbol());
                assert_eq!(cell.fg, Color::Reset);
                assert_eq!(cell.bg, Color::Reset);
            }
        }
    }
}
#[test]
fn tagged_identity_expires_on_pid_reuse_and_replay_disables_alert_actions() {
    let mut app = App::new(Config::default());
    app.replay = true;
    app.ingest(Snapshot {
        processes: vec![proc()],
        ..Default::default()
    });
    app.set_view(View::Processes);
    app.remember_selection();
    app.toggle_tag();
    assert!(app.tagged.contains(&(42, 100)));
    let mut p = proc();
    p.start_time = 101;
    app.ingest(Snapshot {
        processes: vec![p],
        ..Default::default()
    });
    assert!(
        !app.snapshot
            .processes
            .iter()
            .any(|p| app.tagged.contains(&(p.pid, p.start_time)))
    );
}
#[cfg(unix)]
#[test]
fn external_queries_bound_child_descendant_pipes() {
    let start = std::time::Instant::now();
    let result = rstop::discovery::command_partial(
        "sh",
        &["-c", "printf hello; sleep 30 & wait"],
        Duration::from_millis(150),
    )
    .unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(result.contains("hello"));
    assert!(result.contains("partial capture"));
}

#[test]
fn nested_layout_validates_weights_and_handles_missing_panels() {
    let mut config:Config=toml::from_str(r#"
custom_layout = { direction = "vertical", weights = [2, 3], children = [ { direction = "horizontal", children = [{panel="cpu"},{panel="gpu"},{panel="memory"}] }, {panel="processes"} ] }
"#).unwrap();
    config.validate().unwrap();
    let mut app = App::new(config.clone());
    app.ingest(Snapshot::default());
    for size in [(60, 18), (80, 24), (160, 48)] {
        let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
        terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
    }
    config.custom_layout = Some(rstop::config::LayoutNode::Split {
        direction: "horizontal".into(),
        children: vec![rstop::config::LayoutNode::Panel {
            panel: "cpu".into(),
        }],
        weights: vec![0],
    });
    assert!(config.validate().is_err());
}
#[cfg(target_os = "linux")]
#[test]
fn priority_and_affinity_apply_only_to_owned_test_child() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let mut system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(child.id());
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let p = Process {
        pid: child.id(),
        start_time: system.process(pid).unwrap().start_time(),
        ..Default::default()
    };
    let result = (|| -> anyhow::Result<()> {
        rstop::process_tools::priority(&p, 10)?;
        let mut cpus = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        assert_eq!(
            unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&cpus), &mut cpus) },
            0
        );
        let first = (0..libc::CPU_SETSIZE as usize)
            .find(|cpu| unsafe { libc::CPU_ISSET(*cpu, &cpus) })
            .unwrap();
        rstop::process_tools::affinity(&p, &first.to_string())?;
        let status = fs::read_to_string(format!("/proc/{}/status", p.pid))?;
        assert!(
            status
                .lines()
                .any(|line| line.starts_with("Cpus_allowed_list:")
                    && line.split_whitespace().last() == Some(&first.to_string()))
        );
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap();
}
#[cfg(unix)]
#[test]
fn replay_suppresses_commands_and_live_alert_reports_results() {
    let temp = Temp::new("actions");
    let file = temp.0.join("action");
    let config = Config {
        alert_hold_seconds: 0,
        alert_commands: vec![vec![
            "sh".into(),
            "-c".into(),
            "printf done > \"$1\"".into(),
            "rstop-test".into(),
            file.to_string_lossy().into(),
        ]],
        ..Default::default()
    };
    let mut app = App::new(config);
    app.replay = true;
    app.ingest(Snapshot {
        timestamp: 1,
        alerts: vec!["CPU 99%".into()],
        ..Default::default()
    });
    assert!(!file.exists());
    app.replay = false;
    app.ingest(Snapshot {
        timestamp: 2,
        ..Default::default()
    });
    app.ingest(Snapshot {
        timestamp: 3,
        alerts: vec!["CPU 98%".into()],
        ..Default::default()
    });
    let start = std::time::Instant::now();
    while app.action_results.lock().unwrap().is_empty() && start.elapsed() < Duration::from_secs(2)
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(file).unwrap(), "done");
    assert!(!app.action_results.lock().unwrap().is_empty());
}
#[test]
fn platform_cpu_state_counts_accept_native_three_and_four_states() {
    let three = rstop::collector::cpu_breakdown(&[10, 20, 30], &[30, 50, 80]);
    assert_eq!(three["user"], 20.);
    assert_eq!(three["system"], 30.);
    assert_eq!(three["idle"], 50.);
    assert!(!three.contains_key("iowait"));
    let four = rstop::collector::cpu_breakdown(&[0, 0, 0, 0], &[20, 5, 25, 50]);
    assert_eq!(four["nice"], 5.);
    assert_eq!(four.values().sum::<f64>(), 100.);
    assert!(rstop::collector::cpu_breakdown(&[1, 2, 3], &[2, 3]).is_empty());
}

#[test]
fn gpu_history_is_bounded_and_missing_counters_remain_unknown() {
    let mut app = App::new(Config {
        history: 30,
        ..Default::default()
    });
    for timestamp in 0..40 {
        app.ingest(Snapshot {
            timestamp,
            gpus: vec![Gpu {
                id: 0,
                uuid: "gpu-0".into(),
                utilization: Some(70),
                memory_used: Some(1 << 30),
                memory_total: Some(2 << 30),
                power_watts: Some(100.),
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    let h = &app.history.gpu_details["gpu-0"];
    assert_eq!(h.utilization.len(), 30);
    assert_eq!(h.vram.back(), Some(&50.));
    assert!(h.encoder.iter().all(|v| v.is_nan()));
    assert_eq!(h.power.back(), Some(&100.));
    app.ingest(Snapshot::default());
    assert!(app.history.gpu_details.is_empty());
}
#[test]
fn selected_gpu_sort_and_display_use_device_metrics_instead_of_cross_device_totals() {
    let mut a = proc();
    a.pid = 10;
    a.gpu_ids = vec![0, 1];
    a.gpu_memory = Some(1000);
    let mut b = proc();
    b.pid = 20;
    b.gpu_ids = vec![0, 1];
    b.gpu_memory = Some(2000);
    let mut app = App::new(Config::default());
    app.ingest(Snapshot {
        processes: vec![a, b],
        gpus: vec![
            Gpu {
                id: 0,
                uuid: "first".into(),
                processes: vec![
                    GpuProcess {
                        pid: 10,
                        memory: Some(900),
                        utilization: Some(80),
                        ..Default::default()
                    },
                    GpuProcess {
                        pid: 20,
                        memory: Some(100),
                        utilization: Some(10),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            Gpu {
                id: 1,
                uuid: "second".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    app.view = View::Gpu;
    app.sort = rstop::app::Sort::GpuMemory;
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 10);
    assert_eq!(
        app.gpu_metrics(app.selected_process().unwrap()),
        (Some(900), Some(80))
    );
}
#[test]
fn every_gpu_plot_and_sort_menu_renders_across_sizes() {
    let mut app = App::new(Config::default());
    for timestamp in 0..12 {
        app.ingest(Snapshot {
            timestamp,
            gpus: vec![Gpu {
                uuid: "gpu".into(),
                name: "Test accelerator".into(),
                utilization: Some(timestamp as u32 * 5),
                memory_total: Some(100),
                memory_used: Some(60),
                encoder: Some(10),
                decoder: Some(20),
                memory_utilization: Some(30),
                power_watts: Some(100.),
                power_limit_watts: Some(200.),
                temperature: Some(45),
                pcie_rx_kib: Some(100),
                pcie_tx_kib: Some(200),
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    app.view = View::Gpu;
    app.gpu_section = 0;
    for (w, h) in [(60, 18), (80, 24), (100, 30), (160, 48), (240, 70)] {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        if w == 160 {
            let text = term
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            for name in [
                "GPU LOAD",
                "VRAM",
                "ENGINES",
                "POWER",
                "TEMPERATURE",
                "PCIE",
            ] {
                assert!(text.contains(name), "{name}");
            }
        }
        app.ui_menu = Some(rstop::app::UiMenu::new(
            "Sort by",
            rstop::app::Sort::ALL
                .iter()
                .map(|s| (s.name().into(), crossterm::event::KeyCode::Null))
                .collect(),
            17,
        ));
        term.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        app.ui_menu = None;
    }
}

#[test]
fn gpu_focus_keeps_metrics_processes_and_metadata_on_the_same_page() {
    let mut app = App::new(Config::default());
    app.ingest(Snapshot {
        gpus: vec![Gpu {
            vendor: "NVIDIA".into(),
            driver: "595.99.02".into(),
            cuda_driver_version: Some("13.2".into()),
            nvml_version: Some("13.595".into()),
            utilization: Some(80),
            memory_total: Some(1024),
            memory_used: Some(512),
            power_watts: Some(120.),
            temperature: Some(50),
            uuid: "test".into(),
            extra: [
                ("pci_bus_id".into(), serde_json::json!("0000:01:00.0")),
                ("persistence_mode".into(), serde_json::json!(false)),
            ]
            .into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    app.view = View::Gpu;
    for (section, width, height) in [
        (0, 160, 48),
        (1, 160, 48),
        (2, 160, 48),
        (0, 60, 18),
        (1, 60, 18),
        (2, 60, 18),
    ] {
        app.set_gpu_section(section);
        let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
        term.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        let text = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("595.99.02") && text.contains("13.2"));
        assert!(text.contains("PROCESSES") && text.contains("DEVICE INFO"));
        if width == 160 {
            assert!(text.contains("GPU LOAD"));
            assert!(text.contains("TEMPERATURE ·") && text.contains("POWER ·"));
            assert!(text.contains("PCI Bus-ID"));
            assert!(!text.contains("\"pci_bus_id\""));
        }
    }
    let rows = rstop::fields::gpu(&app.snapshot.gpus[0], "persistence");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].2, "Off");
}

#[test]
fn extended_process_fields_use_kernel_units_signed_values_and_numeric_sort() {
    let status = parse_process_fields(
        "VmData: 1024 kB\nUid: 1000 1000 1000 1000\nThreads: 12\nvoluntary_ctxt_switches: 100\nnonvoluntary_ctxt_switches: 20\n",
    );
    assert_eq!(status["VmData"], 1024 * 1024);
    let mut raw = vec!["0"; 38];
    raw[0] = "S";
    raw[4] = "-1";
    raw[13] = "250";
    raw[14] = "125";
    raw[17] = "12";
    raw[36] = "7";
    let stat = parse_process_stat(&format!("42 (name ) with spaces) {}", raw.join(" ")));
    assert_eq!(stat["tty"], -1);
    assert_eq!(stat["processor"], 7);
    let mut a = proc();
    a.extra.insert("status".into(), serde_json::json!(status));
    a.extra.insert("stat".into(), serde_json::json!(stat));
    a.extra
        .insert("clock_ticks_per_second".into(), serde_json::json!(100));
    let mut b = a.clone();
    b.pid = 43;
    b.extra
        .insert("stat".into(), serde_json::json!({"threads":9}));
    let mut app = App::new(Config {
        columns: rstop::fields::PROCESS_COLUMNS
            .iter()
            .map(|c| (*c).into())
            .collect(),
        ..Default::default()
    });
    assert!(app.config.validate().is_ok());
    app.ingest(Snapshot {
        processes: vec![a, b],
        ..Default::default()
    });
    let a = &app.snapshot.processes[0];
    assert_eq!(
        rstop::fields::process_metric(a, &app.snapshot, "cutime"),
        Some(serde_json::json!(2.5))
    );
    assert_eq!(
        rstop::fields::process_metric(a, &app.snapshot, "uid"),
        Some(serde_json::json!(1000))
    );
    assert_eq!(
        rstop::fields::process_metric(a, &app.snapshot, "context_switches"),
        Some(serde_json::json!(120))
    );
    assert_eq!(rstop::fields::process_display(a, &app.snapshot, "pss"), "—");
    app.set_view(View::Processes);
    app.sort = rstop::app::Sort::for_column("threads").unwrap();
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 42);
    app.reverse = true;
    app.selected = 0;
    app.clear_process_selection();
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 43);
}

#[test]
fn taskstats_nested_attributes_and_truncated_records_are_bounded() {
    let mut stats = vec![0u8; 64];
    stats[..2].copy_from_slice(&1u16.to_ne_bytes());
    for (offset, value) in [(24, 3u64), (40, 5), (56, 7)] {
        stats[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
    }
    assert_eq!(rstop::taskstats::parse_delays(&stats), Some([3, 5, 7]));
    assert!(rstop::taskstats::parse_delays(&stats[..63]).is_none());
    let bytes = [7, 0, 3, 128, 9, 8, 7, 0, 255, 255, 0, 0];
    let attrs = rstop::taskstats::attributes(&bytes);
    assert_eq!(attrs, vec![(3, &[9, 8, 7][..])]);
}

#[test]
fn guest_cpu_is_split_without_increasing_total_time() {
    let states = rstop::collector::cpu_breakdown(&[0; 10], &[40, 10, 20, 30, 0, 0, 0, 0, 15, 5]);
    assert_eq!(states["user"], 25.);
    assert_eq!(states["nice"], 5.);
    assert_eq!(states["guest"], 15.);
    assert!((states.values().sum::<f64>() - 100.).abs() < 0.001);
}

#[cfg(target_os = "linux")]
#[test]
fn native_optional_fields_read_own_process_without_enabling_kernel_features() {
    let config = Config {
        columns: rstop::fields::PROCESS_COLUMNS
            .iter()
            .map(|c| (*c).into())
            .collect(),
        ..Default::default()
    };
    let mut sample = Snapshot {
        processes: vec![Process {
            pid: std::process::id(),
            start_time: 1,
            ..Default::default()
        }],
        ..Default::default()
    };
    Advanced::default().collect(&mut sample, &config);
    let p = &sample.processes[0];
    assert!(p.extra["stat"]["threads"].as_u64().unwrap() > 0);
    assert_eq!(p.extra["status"]["Tgid"], std::process::id());
    assert!(p.extra["io"]["rchar"].as_u64().is_some());
    assert!(p.extra["smaps_rollup"]["Pss"].as_u64().unwrap() > 0);
}
