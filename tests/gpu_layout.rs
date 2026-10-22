use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};
use rstop::{
    app::{App, View},
    config::Config,
    model::{Gpu, GpuProcess, Process, Snapshot},
    ui,
};

fn app(devices: u32) -> App {
    let processes: Vec<_> = (0..24)
        .map(|i| Process {
            pid: 100 + i,
            start_time: 10,
            name: format!("worker-{i}"),
            command: format!("python train.py --worker {i}"),
            cpu: 50.,
            gpu_ids: (0..devices).collect(),
            ..Default::default()
        })
        .collect();
    let mut snapshot = Snapshot {
        timestamp: 100,
        processes,
        gpus: (0..devices)
            .map(|id| Gpu {
                id,
                uuid: format!("gpu-{id}"),
                name: "NVIDIA Test GPU".into(),
                vendor: "NVIDIA".into(),
                driver: "595.1".into(),
                utilization: Some(75),
                memory_used: Some(4 << 30),
                memory_total: Some(24 << 30),
                power_watts: Some(200.),
                power_limit_watts: Some(450.),
                temperature: Some(65),
                encoder: Some(3),
                decoder: Some(4),
                memory_utilization: Some(20),
                pcie_rx_kib: Some(100),
                pcie_tx_kib: Some(200),
                fan: Some(40),
                clock_mhz: Some(2100),
                memory_clock_mhz: Some(10000),
                processes: (0..24)
                    .map(|i| GpuProcess {
                        pid: 100 + i,
                        memory: Some(128 << 20),
                        utilization: Some(25),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let mut app = App::new(Config::default());
    app.set_view(View::Gpu);
    for tick in 0..60 {
        snapshot.elapsed = tick as f64;
        app.ingest(snapshot.clone());
    }
    app
}

fn render(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn plot_area(buffer: &Buffer, title: &str) -> Rect {
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        let Some(title_at) = line.find(title) else {
            continue;
        };
        let Some(left) = line[..title_at].rfind('╭') else {
            continue;
        };
        let x = line[..left].chars().count() as u16;
        let right = line[title_at..].find('╮').unwrap() + title_at;
        let width = line[left..right].chars().count() as u16 + 1;
        let bottom = (y + 1..buffer.area.height)
            .find(|next| buffer[(x, *next)].symbol() == "╰")
            .unwrap();
        return Rect::new(x, y, width, bottom - y + 1);
    }
    panic!("Missing plot: {title}");
}

#[test]
fn gpu_info_button_leaves_the_full_width_and_bottom_for_processes_and_details() {
    for devices in [1, 2] {
        for (width, height) in [(160, 48), (275, 73)] {
            let mut app = app(devices);
            render(&mut app, width, height);
            app.move_selection(1);
            assert!(app.open_selection());
            let buffer = render(&mut app, width, height);
            let processes = app.panel_regions.iter().find(|r| r.section == 0).unwrap();
            assert_eq!(processes.area.width, width);
            assert_eq!(processes.area.bottom(), height - 1);
            assert_eq!(app.panel_regions.len(), 1);
            assert!(app.popup_rect.is_empty());
            assert_eq!(app.process_detail_rect.right(), width);
            assert!(app.process_detail_rect.width >= (width * 2 / 5).min(90));
            assert!(app.process_detail_rect.height >= 14);
            assert!(app.table_rect.right() < app.process_detail_rect.x);
            let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
            assert!(text.contains("PID 100"));
            assert!(text.contains("Since selection"));
            assert!(text.contains("[I] Device info"));
            assert!(!text.contains("DEVICE INFO"));
        }
    }
}

#[test]
fn gpu_charts_have_room_for_curves_and_keep_all_supported_counters_on_wide_screens() {
    for devices in [1, 2] {
        let mut app = app(devices);
        let buffer = render(&mut app, 160, 48);
        for title in [
            "GPU LOAD",
            "VRAM (",
            "ENGINES",
            "POWER (",
            "TEMPERATURE",
            "PCIE",
            "FAN (",
            "CLOCKS",
        ] {
            let area = plot_area(&buffer, title);
            assert!(area.height >= 6, "{devices} GPUs: {title} is only {area:?}");
            assert!(area.width >= 38, "{title}: {area:?}");
        }
    }
}

#[test]
fn multiple_gpus_stack_vertically_and_follow_the_selected_device_after_resizing() {
    for devices in [2, 8] {
        let mut app = app(devices);
        for selected in [0, devices as usize / 2, devices as usize - 1] {
            app.selected_gpu = selected;
            for (width, height) in [(60, 18), (80, 24), (120, 36), (160, 48), (275, 73)] {
                render(&mut app, width, height);
                assert!(app.gpu_ranges.iter().any(|(_, id)| *id == selected));
                for (area, _) in &app.gpu_ranges {
                    assert_eq!(area.x, 0);
                    assert_eq!(area.width, width);
                }
                for pair in app.gpu_ranges.windows(2) {
                    assert!(pair[0].0.bottom() <= pair[1].0.y);
                    assert_eq!(pair[0].1 + 1, pair[1].1);
                }
                if height >= 36 {
                    assert!(app.gpu_ranges.len() >= 2);
                }
                if height >= 73 {
                    assert_eq!(app.gpu_ranges.len(), devices as usize);
                }
                for region in &app.panel_regions {
                    assert!(region.table.height >= 1);
                }
            }
        }
    }
}

#[test]
fn device_info_final_field_and_scrollbar_reach_the_bottom_together() {
    let mut app = app(1);
    app.snapshot.gpus[0].extra.insert(
        "nvlink_zz_end_marker".into(),
        serde_json::json!("DEVICE_FIELDS_END"),
    );
    app.focus_panel(2);
    for (width, height) in [(60, 18), (80, 24), (120, 36), (160, 48), (275, 73)] {
        render(&mut app, width, height);
        app.move_selection(isize::MAX);
        let buffer = render(&mut app, width, height);
        let area = app.table_rect;
        let last_line = (area.x..area.right())
            .map(|x| buffer[(x, area.bottom() - 1)].symbol())
            .collect::<String>();
        assert!(
            last_line.contains("DEVICE_FIELDS_END"),
            "{width}x{height}: {last_line}"
        );
        assert_eq!(
            buffer[(area.right(), area.bottom() - 1)].symbol(),
            "█",
            "{width}x{height}: device scrollbar stops above final row"
        );
    }
}
