use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Color};
use rstop::{
    app::{App, View},
    config::Config,
    model::{
        Battery, Container, Disk, DiskIo, Gpu, Inventory, Memory, Network, Pressure, Process,
        Sensor, Service, Snapshot,
    },
    ui,
};

fn sample() -> Snapshot {
    Snapshot {
        timestamp: 100,
        hostname: "visual-host".into(),
        cpu_percent: 35.,
        cores: vec![5., 35., 75., 100.],
        cpu_breakdown: [("user".into(), 25.), ("system".into(), 10.)].into(),
        memory: Memory {
            total: 8 << 30,
            used: 4 << 30,
            available: 4 << 30,
            ..Default::default()
        },
        disks: vec![
            Disk {
                mount: "/half".into(),
                total: 1000,
                used: 500,
                ..Default::default()
            },
            Disk {
                mount: "/empty".into(),
                total: 1000,
                used: 0,
                ..Default::default()
            },
            Disk {
                mount: "/unknown".into(),
                total: 0,
                ..Default::default()
            },
        ],
        disk_io: vec![DiskIo {
            name: "nvme0n1".into(),
            busy_percent: Some(50.),
            aggregate: true,
            ..Default::default()
        }],
        sensors: vec![Sensor {
            name: "CPU package".into(),
            temperature: 98.5,
            critical: Some(95.),
        }],
        batteries: vec![Battery {
            name: "BAT0".into(),
            percent: None,
            status: "Unknown".into(),
            ..Default::default()
        }],
        pressure: vec![Pressure {
            resource: "memory".into(),
            some_10: Some(0.),
            full_10: None,
            ..Default::default()
        }],
        processes: vec![Process {
            pid: 42,
            start_time: 10,
            name: "worker".into(),
            command: "worker".into(),
            cpu: 325.,
            memory: 2 << 30,
            state: "Run".into(),
            ..Default::default()
        }],
        inventory: Inventory {
            updated_at: 100,
            runtimes: vec!["docker".into()],
            service_managers: vec!["systemd/system".into()],
            containers: vec![
                Container {
                    id: "a".into(),
                    name: "known".into(),
                    state: "running".into(),
                    cpu: Some(0.),
                    memory: Some(1000),
                    memory_limit: Some(2000),
                    ..Default::default()
                },
                Container {
                    id: "b".into(),
                    name: "missing".into(),
                    state: "exited".into(),
                    ..Default::default()
                },
            ],
            services: vec![
                Service {
                    name: "healthy.service".into(),
                    active: "active".into(),
                    cpu_percent: Some(0.),
                    ..Default::default()
                },
                Service {
                    name: "broken.service".into(),
                    active: "failed".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn render(view: View, config: Config, width: u16, height: u16) -> Buffer {
    let mut app = App::new(config);
    app.ingest(sample());
    app.set_view(view);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    terminal.backend().buffer().clone()
}
fn lines(buffer: &Buffer) -> Vec<String> {
    buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect())
        .collect()
}

#[test]
fn overview_preserves_resource_metrics_and_gives_charts_most_of_the_screen() {
    let core_pattern = regex::Regex::new(r"C(\d{2}) ([\u{2800}-\u{28ff} ]+)\s+50\.0%").unwrap();
    let joined_percent_pattern = regex::Regex::new(r"%\d").unwrap();
    for (width, height) in [(120, 40), (160, 48), (240, 70)] {
        let mut snapshot = sample();
        snapshot.cpu_name = "32-core test CPU".into();
        snapshot.cpu_mhz = 4000;
        snapshot.cores = vec![50.; 32];
        snapshot.memory.cached = Some(3 << 30);
        snapshot.memory.free = Some(1 << 30);
        snapshot.load = [1.25, 2.50, 3.75];
        snapshot.memory.swap_used = 1 << 30;
        snapshot.memory.swap_total = 2 << 30;
        snapshot.gpus = vec![Gpu {
            name: "Test GPU".into(),
            vendor: "NVIDIA".into(),
            backend: "NVML".into(),
            uuid: "test-gpu".into(),
            utilization: Some(75),
            memory_used: Some(4 << 30),
            memory_total: Some(16 << 30),
            temperature: Some(70),
            power_watts: Some(145.),
            fan: Some(40),
            ..Default::default()
        }];
        snapshot.networks = vec![Network {
            name: "eth0".into(),
            rx_per_sec: 128. * 1024.,
            tx_per_sec: 64. * 1024.,
            rx_total: 3 << 30,
            tx_total: 5 << 30,
            ..Default::default()
        }];
        let mut second_gpu = snapshot.gpus[0].clone();
        second_gpu.id = 1;
        second_gpu.uuid = "second-test-gpu".into();
        snapshot.gpus.push(second_gpu);
        let mut app = App::new(Config::default());
        for index in 0..180 {
            snapshot.elapsed = index as f64;
            for (core, value) in snapshot.cores.iter_mut().enumerate() {
                *value = if index == 179 {
                    50.
                } else {
                    (index * (core % 3 + 1) % 100) as f32
                };
            }
            app.ingest(snapshot.clone());
        }
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        let rows = lines(terminal.backend().buffer());
        let text = rows.join("\n");
        for metric in [
            "4000 MHz",
            "32 cores",
            "CPU history",
            "Temp 98.5°C",
            "AVG 1m 1.25 · 5m 2.50 · 15m 3.75",
            "Used 4.0 GiB",
            "Available 4.0 GiB",
            "Cached 3.0 GiB",
            "Free 1.0 GiB",
            "Swap 1.0 GiB / 2.0 GiB",
            "Test GPU",
            "4.0 GiB / 16.0 GiB",
            "70°C",
            "145 W",
            "fan 40%",
            "NVML",
            "1/2",
            "/half",
            "500 B / 1000 B",
            "1 interfaces",
            "128.0 KiB/s",
            "64.0 KiB/s",
            "3.0 GiB",
            "5.0 GiB",
            "↓ Top 128.0 KiB/s",
            "↑ Top 64.0 KiB/s",
            "Download history",
            "Upload history",
        ] {
            assert!(
                text.contains(metric),
                "{width}x{height}: missing {metric}\n{text}"
            );
        }
        assert_eq!(app.table_rect.height, 0);
        assert!(!app.has_process_table());
        assert!(!text.contains("PROCESSES"));
        let core_rows: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| core_pattern.is_match(row))
            .collect();
        assert_eq!(core_rows.len(), 16, "Cores must occupy two columns: {text}");
        assert!(
            core_rows
                .iter()
                .all(|(_, row)| core_pattern.captures_iter(row).count() == 2),
            "{text}"
        );
        let memory_end = rows
            .iter()
            .find(|row| row.contains("╭ MEMORY"))
            .unwrap()
            .chars()
            .position(|c| c == '╮')
            .unwrap();
        let memory_text = |row: &String| row.chars().take(memory_end).collect::<String>();
        for (metric, next) in [
            ("Used", "Available"),
            ("Available", "Cached"),
            ("Cached", "Free"),
            ("Free", "Swap"),
        ] {
            let row = rows
                .iter()
                .position(|row| memory_text(row).contains(metric))
                .unwrap();
            let end = rows
                .iter()
                .skip(row + 1)
                .position(|row| memory_text(row).contains(next))
                .unwrap()
                + row
                + 1;
            assert!(
                rows[row + 1..end].iter().any(|row| memory_text(row)
                    .chars()
                    .any(|c| ('\u{2800}'..='\u{28ff}').contains(&c))),
                "Missing {metric} curve: {text}"
            );
        }
        if width >= 160 {
            assert!(text.contains("GPU history") && text.contains("VRAM history"));
            assert!(text.contains("Read history") && text.contains("Write history"));
            let cores: Vec<_> = core_rows
                .iter()
                .flat_map(|(_, row)| core_pattern.captures_iter(row))
                .collect();
            assert_eq!(cores.len(), 32, "{text}");
            let traces: std::collections::BTreeSet<_> =
                cores.iter().map(|core| core[2].trim()).collect();
            assert!(
                traces.len() >= 3,
                "Each core must use its own history: {text}"
            );
            assert!(
                cores.iter().all(|core| core[2].chars().count() >= 10),
                "{text}"
            );
            assert!(
                cores.iter().all(|core| core[2]
                    .chars()
                    .filter(|c| ('\u{2800}'..='\u{28ff}').contains(c))
                    .all(|c| (c as u32 - 0x2800) & 0x09 == 0)),
                "Core graphs must leave a visible gap: {text}"
            );
            let history_row = rows
                .iter()
                .position(|row| row.contains("CPU history"))
                .unwrap();
            assert!(history_row < core_rows[0].0);
        }
        let card_row = rows
            .iter()
            .position(|row| row.contains("╭ MEMORY"))
            .unwrap();
        assert!(
            rows[card_row].contains("╭ GPU")
                && rows[card_row].contains("╭ STORAGE")
                && rows[card_row].contains("╭ NETWORK")
        );
        if width == 160 {
            let cpu_row = rows.iter().position(|row| row.contains("╭ CPU")).unwrap();
            assert!(
                (20..=22).contains(&(card_row - cpu_row - 1)),
                "CPU must have room for two columns of 16 cores: {text}"
            );
        }
        assert!(!joined_percent_pattern.is_match(&text));
        let gpu_row = rows
            .iter()
            .position(|row| row.contains(" GPU ") && row.contains("75.0%"))
            .unwrap();
        let vram_row = rows
            .iter()
            .position(|row| row.contains(" VRAM") && row.contains("25.0%"))
            .unwrap();
        assert!(vram_row >= gpu_row + 2);
    }
}

#[test]
fn capacity_text_distinguishes_real_zero_from_unknown_without_inline_graphics() {
    let buffer = render(View::Storage, Config::default(), 160, 48);
    let rows = lines(&buffer);
    let half = rows.iter().find(|r| r.contains("/half")).unwrap();
    let empty = rows.iter().find(|r| r.contains("/empty")).unwrap();
    let unknown = rows.iter().find(|r| r.contains("/unknown")).unwrap();
    assert!(half.contains("50.0%"));
    assert!(empty.contains("0.0%"));
    assert!(unknown.contains('—') && !unknown.contains("0.0%"));
    for row in [half, empty, unknown] {
        assert!(!row.contains(['█', '░', '┄', '▏', '▍', '▌', '▋', '▊', '▉']));
    }
}

#[test]
fn visual_process_metrics_preserve_multicore_cpu_and_unknown_gpu() {
    let buffer = render(
        View::Processes,
        Config {
            columns: vec![
                "pid".into(),
                "cpu".into(),
                "memory".into(),
                "gpu".into(),
                "command".into(),
            ],
            ..Default::default()
        },
        160,
        48,
    );
    let rows = lines(&buffer);
    let worker = rows.iter().find(|r| r.contains("worker")).unwrap();
    assert!(worker.contains("325.0"), "{worker}");
    assert!(
        !worker.contains(['█', '░', '┄']),
        "Table metrics are text only: {worker}"
    );
    assert!(worker.contains('—'), "Unknown GPU is not idle: {worker}");
}

#[test]
fn state_text_uses_color_and_resource_pages_work_in_small_and_accessible_modes() {
    let services = render(View::Services, Config::default(), 160, 48);
    let rows = lines(&services);
    assert!(
        rows.iter()
            .any(|r| r.contains("healthy.service") && r.contains("active"))
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("broken.service") && r.contains("failed"))
    );
    let palette = ui::palette("midnight");
    for (name, state, color) in [
        ("healthy.service", "active", palette.mint),
        ("broken.service", "failed", palette.red),
    ] {
        let (y, row) = rows
            .iter()
            .enumerate()
            .find(|(_, r)| r.contains(name))
            .unwrap();
        assert!(!row.contains(['●', '■', '◇', '◆', '○', '◉', '█', '░', '┄']));
        let x = row[..row.find(state).unwrap()].chars().count();
        assert_eq!(services[(x as u16, y as u16)].fg, color);
    }
    for (width, height) in [(60, 18), (80, 24), (100, 30), (160, 48)] {
        for view in [
            View::Overview,
            View::Processes,
            View::Storage,
            View::System,
            View::Containers,
            View::Services,
        ] {
            let buffer = render(
                view,
                Config {
                    ascii: true,
                    no_color: true,
                    ..Default::default()
                },
                width,
                height,
            );
            assert!(
                buffer
                    .content
                    .iter()
                    .all(|c| c.symbol().is_ascii() && c.fg == Color::Reset && c.bg == Color::Reset)
            );
            if view == View::Storage {
                assert!(lines(&buffer).iter().any(|r| r.contains("CAPACITY")));
            }
        }
    }
}

#[test]
fn overview_uses_cpu_sensors_and_package_power_without_subdomain_double_counting() {
    let mut app = App::new(Config::default());
    let mut snapshot = sample();
    snapshot.cpu_name = "Sensor test CPU".into();
    snapshot.cpu_mhz = 3200;
    snapshot.sensors = vec![
        Sensor {
            name: "nvme Composite".into(),
            temperature: 95.,
            critical: None,
        },
        Sensor {
            name: "amdgpu edge".into(),
            temperature: 99.,
            critical: None,
        },
        Sensor {
            name: "k10temp Tctl".into(),
            temperature: 62.,
            critical: None,
        },
        Sensor {
            name: "k10temp Tdie".into(),
            temperature: 60.,
            critical: None,
        },
    ];
    snapshot.extra.insert(
        "CPU power (RAPL)".into(),
        serde_json::json!([
            {"name":"package-0", "watts":40.},
            {"name":"package-1", "watts":50.},
            {"name":"core", "watts":20.},
            {"name":"dram", "watts":10.}
        ]),
    );
    app.ingest(snapshot.clone());
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    let text = lines(terminal.backend().buffer()).join("\n");
    assert!(
        text.contains("Clock 3200 MHz · Temp 60.0°C · 90.0 W"),
        "{text}"
    );
    snapshot
        .sensors
        .retain(|sensor| !sensor.name.starts_with("k10temp"));
    app.ingest(snapshot);
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    let text = lines(terminal.backend().buffer()).join("\n");
    assert!(
        text.contains("Temp —"),
        "Do not label disk/GPU temperature as CPU: {text}"
    );
    assert!(text.contains("Free —") && text.contains("Cached —"));
}

#[test]
fn ascii_and_no_color_preserve_the_program_identity_in_the_header() {
    for layout in ["balanced", "compute", "storage", "minimal"] {
        let buffer = render(
            View::Overview,
            Config {
                layout: layout.into(),
                ascii: true,
                no_color: true,
                ..Default::default()
            },
            120,
            40,
        );
        assert!(lines(&buffer)[0].trim_start().starts_with("rstop"));
        assert!(buffer.content.iter().all(|cell| cell.symbol().is_ascii()
            && cell.fg == Color::Reset
            && cell.bg == Color::Reset));
    }
}
