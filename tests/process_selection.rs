use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, Sort, View},
    config::Config,
    model::{Gpu, GpuProcess, Process, Snapshot},
    ui,
};

fn snapshot(tick: u64) -> Snapshot {
    Snapshot {
        timestamp: tick,
        elapsed: tick as f64,
        cpu_percent: 12.,
        processes: (0..50)
            .map(|i| Process {
                pid: 100 + i,
                start_time: 1000 + i as u64,
                name: format!("task-{i}"),
                cpu: (50 - i) as f32 + tick as f32,
                memory: 1024 * (i as u64 + 1),
                gpu_ids: vec![0],
                gpu_memory: Some(9999),
                gpu_percent: Some(99),
                ..Default::default()
            })
            .collect(),
        gpus: vec![Gpu {
            id: 0,
            uuid: "gpu-test".into(),
            vendor: "NVIDIA".into(),
            name: "Test GPU".into(),
            utilization: Some(90),
            memory_used: Some(1024),
            memory_total: Some(2048),
            power_watts: Some(120.),
            power_limit_watts: Some(200.),
            temperature: Some(65),
            encoder: Some(3),
            decoder: Some(4),
            memory_utilization: Some(20),
            pcie_rx_kib: Some(100),
            pcie_tx_kib: Some(200),
            processes: (0..50)
                .map(|i| GpuProcess {
                    pid: 100 + i,
                    memory: Some(128),
                    utilization: Some(25),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn hidden_gpu_charts_keep_real_time_samples_when_switching_pages() {
    let mut app = App::new(Config::default());
    app.set_view(View::Gpu);
    for tick in 0..10 {
        if tick == 2 {
            app.set_view(View::Services);
        }
        let mut sample = snapshot(tick);
        sample.gpus[0].utilization = Some(tick as u32);
        app.ingest(sample);
    }
    app.set_view(View::Gpu);
    let gpu = &app.history.gpu_details["gpu-test"];
    assert_eq!(
        gpu.utilization.iter().copied().collect::<Vec<_>>(),
        (0..10).map(f64::from).collect::<Vec<_>>()
    );
    assert_eq!(
        app.history.timestamps.iter().copied().collect::<Vec<_>>(),
        (0..10).map(f64::from).collect::<Vec<_>>()
    );
    assert_eq!(gpu.power.len(), 10);
    assert_eq!(app.history.cpu.len(), 10);
}

#[test]
fn selection_is_explicit_keeps_order_and_does_not_transfer_to_reused_pids() {
    let mut app = App::new(Config::default());
    app.set_view(View::Processes);
    app.ingest(snapshot(0));
    assert!(app.selected_process().is_none());
    app.selected = 3;
    app.remember_selection();
    assert_eq!(app.selected_process().unwrap().pid, 103);
    let before = app.process_rows();
    let mut next = snapshot(1);
    next.processes[30].cpu = 1000.;
    app.ingest(next);
    assert_eq!(app.process_rows(), before);
    assert_eq!(app.selected, 3);
    assert_eq!(app.selected_process().unwrap().cpu, 48.);
    assert_eq!(app.process_history.cpu.len(), 2);
    app.clear_process_selection();
    app.ingest(snapshot(2));
    assert!(app.selected_process().is_none());
    assert!(app.process_history.cpu.is_empty());
    app.move_selection(1);
    let id = app.selected_process.unwrap();
    let mut reused = snapshot(3);
    reused
        .processes
        .iter_mut()
        .find(|p| p.pid == id.0)
        .unwrap()
        .start_time += 1;
    app.ingest(reused);
    assert!(app.selected_process().is_none());
    assert!(app.process_history.cpu.is_empty());
}

#[test]
fn selecting_a_new_process_holds_its_position_when_more_processes_arrive() {
    let mut app = App::new(Config::default());
    app.set_view(View::Processes);
    app.ingest(snapshot(0));
    app.selected = 3;
    app.remember_selection();
    let mut next = snapshot(1);
    next.processes.push(Process {
        pid: 999,
        start_time: 1,
        cpu: 1000.,
        ..Default::default()
    });
    app.ingest(next.clone());
    app.selected = app
        .process_rows()
        .iter()
        .position(|(i, _)| app.snapshot.processes[*i].pid == 999)
        .unwrap();
    app.remember_selection();
    let selected_row = app.selected;
    next.timestamp += 1;
    next.processes.last_mut().unwrap().cpu = 1.;
    next.processes.push(Process {
        pid: 1000,
        start_time: 1,
        cpu: 2000.,
        ..Default::default()
    });
    app.ingest(next);
    assert_eq!(app.selected, selected_row);
    assert_eq!(app.selected_process().unwrap().pid, 999);
    assert_eq!(app.selected_process().unwrap().cpu, 1.);
}

#[test]
fn selected_histories_are_bounded_reset_on_target_change_and_use_device_attribution() {
    let mut app = App::new(Config {
        history: 30,
        ..Default::default()
    });
    app.ingest(snapshot(0));
    app.set_view(View::Gpu);
    app.remember_selection();
    for tick in 1..80 {
        app.ingest(snapshot(tick));
    }
    assert_eq!(app.process_history.cpu.len(), 30);
    assert_eq!(*app.process_history.cpu.back().unwrap(), 129.);
    assert_eq!(*app.process_history.gpu.back().unwrap(), 25.);
    assert_eq!(*app.process_history.vram.back().unwrap(), 128.);
    app.move_selection(1);
    assert_eq!(app.process_history.cpu.len(), 1);
    let mut missing = snapshot(80);
    missing.gpus[0].processes.clear();
    app.ingest(missing);
    assert!(app.process_history.gpu.back().unwrap().is_nan());
    assert!(app.process_history.vram.back().unwrap().is_nan());
}

#[test]
fn selected_process_pane_keeps_the_inline_filter_readable() {
    let mut app = App::new(Config::default());
    app.set_view(View::Processes);
    app.ingest(snapshot(0));
    app.set_view(View::Processes);
    app.filter = "task".into();
    app.sort = Sort::Read;
    app.remember_selection();
    assert!(app.open_selection());
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("Filter: task"));
    assert!(text.contains("Selected"));
    assert!(text.contains("[Esc] Back"));
}

#[test]
fn full_gpu_summary_and_live_process_focus_render_on_small_and_wide_terminals() {
    let mut app = App::new(Config::default());
    app.set_view(View::Processes);
    app.ingest(snapshot(0));
    for (width, height) in [(60, 18), (80, 24), (160, 48), (275, 73)] {
        for view in [View::Overview, View::Processes, View::Gpu] {
            app.set_view(view);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            if view == View::Gpu && width >= 160 {
                let text = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                for label in [
                    "GPU LOAD",
                    "VRAM",
                    "POWER",
                    "TEMPERATURE",
                    "ENGINES",
                    "PCIE",
                ] {
                    assert!(text.contains(label), "{label}");
                }
            }
            app.move_selection(1);
            if app.has_process_table() {
                assert!(app.open_selection());
            }
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            if app.process_detail_rect.height > 0 {
                assert!(text.contains("PID 100"));
                assert!(text.contains("CPU 50.0%"));
            }
            if width >= 160 && view == View::Processes {
                assert!(text.contains("Since selection"));
            }
            if view == View::Overview {
                assert_eq!(app.process_detail_rect.height, 0);
                assert_eq!(app.table_rect.height, 0);
                assert!(!app.has_process_table());
                assert!(app.selected_process().is_none());
            }
        }
    }
}
