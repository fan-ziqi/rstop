mod control;
mod input;
mod inspect;
mod inventory;
mod sampling;
use anyhow::Result;
#[cfg(all(test, target_os = "linux"))]
use control::send_signal;
#[cfg(windows)]
use crossterm::event::EnableMouseCapture;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyEventKind},
    execute,
};
use inspect::load_logs;
use rstop::{STOP, app::App, collector::Collector, config::Config, ui};
#[cfg(unix)]
use std::io::Write;
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub fn run(args: &crate::Args, config: Config, path: PathBuf) -> Result<()> {
    let initial_snapshot = args
        .replay
        .is_none()
        .then(rstop::collector::initial_snapshot);
    let interval = Arc::new(AtomicU64::new(config.interval_ms));
    let preferences = Arc::new(RwLock::new(config.clone()));
    let full_sampling = args.full_sampling || args.record.is_some();
    let sources = Arc::new(sampling::Sources::new(full_sampling));
    let initial_view = crate::wanted_view(args, &config);
    sources.activate(
        initial_view,
        initial_view == rstop::app::View::Overview
            && config
                .custom_layout
                .as_ref()
                .is_some_and(|l| l.contains_panel("processes")),
    );
    let initial_inventory = inventory::Cache::default();
    let inventory_revision = initial_inventory.revision.clone();
    let inventory = Arc::new(RwLock::new(initial_inventory));
    let connection_worker = args.replay.is_none().then(|| {
        let sources = sources.clone();
        rstop::connections::worker_with_demand(move || {
            let n = sources.needs();
            rstop::connections::Demand {
                owners: n.gpu_processes,
                sockets: n.connections,
            }
        })
    });
    let connections = connection_worker.as_ref().map(|(cache, _)| cache.clone());
    let discovery_workers = inventory::workers(
        inventory.clone(),
        preferences.clone(),
        sources.clone(),
        args.replay.is_none(),
    )?;
    let (tx, rx) = mpsc::sync_channel(1);
    let replay_data = args
        .replay
        .as_ref()
        .map(|p| rstop::recording::replay(p))
        .transpose()?;
    let replay_len = replay_data.as_ref().map_or(0, Vec::len);
    let replay_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut recorder = args
        .record
        .as_ref()
        .map(|p| rstop::recording::Recorder::new(p, config.record_max_mib))
        .transpose()?;
    let sampler = {
        let sources = sources.clone();
        let interval = interval.clone();
        let inventory = inventory.clone();
        let preferences = preferences.clone();
        let mut config = config.clone();
        let gpu = config.collect_gpu;
        let replay_index = replay_index.clone();
        thread::Builder::new()
            .name("rstop-sampler".into())
            .spawn(move || {
                let mut collector = if replay_data.is_none() {
                    Some(Collector::new(gpu))
                } else {
                    None
                };
                let cpu_ready = Instant::now() + sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;
                let mut process_cpu_ready = None;
                let mut recording_error = None::<String>;
                while !STOP.load(Ordering::Relaxed) {
                    let began = Instant::now();
                    if let Ok(current) = preferences.read() {
                        config = current.clone();
                    }
                    config.interval_ms = interval.load(Ordering::Relaxed);
                    let needed = sources.needs();
                    let mut data = if let Some(snapshots) = &replay_data {
                        let index =
                            replay_index.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i| {
                                (i < snapshots.len()).then_some(i + 1)
                            });
                        let Ok(index) = index else {
                            crate::wait_until(Instant::now() + Duration::from_millis(100));
                            continue;
                        };
                        snapshots[index].clone()
                    } else {
                        collector
                            .as_mut()
                            .expect("live collector")
                            .collect_selected(&config, needed.processes, needed.gpu_processes)
                    };
                    let mut sampled_revision = 0;
                    if replay_data.is_none()
                        && let Ok(current) = inventory.read()
                    {
                        sampled_revision = current.revision.load(Ordering::Relaxed);
                        data.apply_inventory(current.snapshot());
                    }
                    if let Some(cache) = &connections
                        && let Ok(sample) = cache.read()
                    {
                        data.connections = sample.clone();
                        for row in &mut data.connections.connections {
                            if row.process_names.is_empty() {
                                row.process_names = row
                                    .pids
                                    .iter()
                                    .filter_map(|pid| {
                                        data.processes
                                            .iter()
                                            .find(|p| p.pid == *pid)
                                            .map(|p| p.name.clone())
                                    })
                                    .collect();
                            }
                        }
                    }
                    let applications = rstop::tags::annotate(&mut data);
                    if let Some(writer) = &mut recorder
                        && let Err(error) = writer.write(&data)
                    {
                        recording_error = Some(error.to_string());
                        recorder = None;
                    }
                    if let Some(error) = &recording_error {
                        data.extra
                            .insert("Recording error".into(), serde_json::json!(error));
                    }
                    if matches!(
                        tx.try_send((data, applications)),
                        Err(mpsc::TrySendError::Disconnected(_))
                    ) {
                        break;
                    }
                    // Publish a baseline immediately, then warm up process CPU rates
                    // before waiting a full refresh period.
                    if replay_data.is_none() && needed.processes && process_cpu_ready.is_none() {
                        process_cpu_ready =
                            Some(Instant::now() + sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
                    }
                    let ready = process_cpu_ready.unwrap_or(cpu_ready).max(cpu_ready);
                    let until = if began < ready && replay_data.is_none() {
                        ready
                    } else {
                        began + Duration::from_millis(config.interval_ms)
                    };
                    while Instant::now() < until
                        && sources.needs() == needed
                        && inventory_revision.load(Ordering::Relaxed) == sampled_revision
                        && !STOP.load(Ordering::Relaxed)
                    {
                        thread::park_timeout(Duration::from_millis(20));
                    }
                }
            })?
    };
    let guard = TerminalGuard;
    let mut terminal = ratatui::try_init()?;
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        STOP.store(true, Ordering::Relaxed);
        disable_mouse_capture();
        panic_hook(info);
    }));
    enable_mouse_capture()?;
    let mut app = App::new(config.clone());
    if let Some(snapshot) = initial_snapshot {
        app.snapshot = snapshot;
    }
    app.set_view(crate::wanted_view(args, &config));
    app.gpu_section = crate::gpu_section(args);
    app.replay = args.replay.is_some();
    let mut first = true;
    let (detail_tx, detail_rx) = mpsc::channel();
    let (action_tx, action_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();
    let update_worker = (config.check_updates && !args.read_only && !app.replay)
        .then(|| rstop::updates::check_background(update_tx.clone()))
        .flatten();
    let mut pending_update = None;
    let mut seen_updates = std::collections::HashSet::new();
    let mut dirty = true;
    let mut log_refresh = Instant::now();
    let mut input_state = input::Input {
        interval: &interval,
        detail_tx: detail_tx.clone(),
        action_tx: action_tx.clone(),
        update_tx,
        path: &path,
        read_only: args.read_only,
        replay_index: &replay_index,
        replay_len,
        queued_key: None,
        last_click: None,
        last_escape: None,
        ros_job: None,
    };
    let result = (|| -> Result<()> {
        while !STOP.load(Ordering::Relaxed) {
            while let Ok(event) = update_rx.try_recv() {
                match event {
                    rstop::updates::Event::Available(release) => {
                        if app.config.ignored_update_version.as_deref() != Some(release.version())
                            && seen_updates.insert(release.version().to_owned())
                        {
                            pending_update = Some(release);
                        }
                    }
                    rstop::updates::Event::Installed(result) => {
                        if let Some(notice) = &mut app.update_notice {
                            notice.phase = match result {
                                Ok(version) => rstop::updates::Phase::Installed(version),
                                Err(error) => rstop::updates::Phase::Failed(error),
                            };
                            notice.selected = 0;
                        }
                        dirty = true;
                    }
                }
            }
            if pending_update.is_some()
                && app.update_notice.is_none()
                && !app.editing_filter
                && !app.help
                && !app.settings
                && app.detail.is_none()
                && app.ui_menu.is_none()
                && app.main_menu.is_none()
                && app.prompt.is_none()
                && app.action_menu.is_none()
                && app.signal.is_none()
            {
                app.update_notice = pending_update.take().map(rstop::updates::Notice::new);
                input_state.queued_key = None;
                input_state.last_click = None;
                dirty = true;
            }
            if sources.activate(app.view, app.has_process_table()) {
                sampler.thread().unpark();
                for worker in &discovery_workers {
                    worker.thread().unpark();
                }
                if let Some((_, worker)) = &connection_worker {
                    worker.thread().unpark();
                }
            }
            app.config.sort_index = app.sort.stored();
            app.config.reverse_sort = app.reverse;
            app.config.tree = app.tree;
            if let Ok(mut current) = preferences.write() {
                *current = app.config.clone();
            }
            while let Ok((snapshot, applications)) = rx.try_recv() {
                app.ingest_annotated(snapshot, applications);
                if first {
                    first = false;
                    app.status.clear();
                }
                dirty = true;
            }
            if app.follow_logs
                && app.detail.is_some()
                && !app.detail_pending
                && log_refresh.elapsed() > Duration::from_secs(3)
            {
                load_logs(&mut app, detail_tx.clone());
                log_refresh = Instant::now();
            }
            dirty |= input_state.poll_ros(&mut app);
            while let Ok(message) = action_rx.try_recv() {
                if let Some(menu) = &mut app.main_menu {
                    menu.status = message.clone();
                }
                app.status = message;
                dirty = true;
            }
            while let Ok((generation, title, body)) = detail_rx.try_recv() {
                if generation != app.detail_generation {
                    continue;
                }
                app.detail_pending = false;
                app.detail_scroll = if app.follow_logs {
                    let count = if app.detail_filter.is_empty() {
                        body.lines().count()
                    } else {
                        body.lines()
                            .filter(|line| {
                                line.to_lowercase()
                                    .contains(&app.detail_filter.to_lowercase())
                            })
                            .count()
                    };
                    count.saturating_sub(app.detail_rows)
                } else {
                    0
                };
                app.detail = Some((title, body));
                if app.log_target.is_some() {
                    app.status.clear();
                }
                dirty = true;
            }
            if dirty {
                terminal.draw(|frame| ui::draw(frame, &mut app))?;
                dirty = false;
            }
            // Queued controls dispatch actions without counting as physical Esc presses.
            if let Some(key) = input_state.queued_key.take() {
                dirty = true;
                if !input_state.key(&mut app, key) {
                    break;
                }
                continue;
            }
            if !event::poll(Duration::from_millis(80))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    dirty = true;
                    if !input_state.terminal_key(&mut app, key, Instant::now()) {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    dirty = true;
                    input_state.mouse(&mut app, mouse);
                }
                Event::Resize(_, _) => dirty = true,
                Event::FocusLost => input_state.last_escape = None,
                _ => {}
            }
        }
        Ok(())
    })();
    STOP.store(true, Ordering::Relaxed);
    if let Some(worker) = update_worker {
        worker.thread().unpark();
    }
    sampler.thread().unpark();
    for worker in &discovery_workers {
        worker.thread().unpark();
    }
    if let Some((_, worker)) = &connection_worker {
        worker.thread().unpark();
    }
    drop(terminal);
    drop(guard);
    let _ = sampler.join();
    for worker in discovery_workers {
        let _ = worker.join();
    }
    if let Some((_, worker)) = connection_worker {
        let _ = worker.join();
    }
    result
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        disable_mouse_capture();
        ratatui::restore();
    }
}

fn enable_mouse_capture() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut stdout = io::stdout().lock();
        execute!(stdout, DisableMouseCapture)?;
        // Only clicks and wheel events are used; reporting every pointer movement
        // needlessly fills the input queue and can spill into the shell on exit.
        stdout.write_all(b"\x1b[?1000h\x1b[?1006h")?;
        stdout.flush()
    }
    #[cfg(windows)]
    {
        execute!(io::stdout(), EnableMouseCapture)
    }
}

fn disable_mouse_capture() {
    let _ = execute!(io::stdout(), DisableMouseCapture);
    // Disabling reports does not remove bytes already queued by the terminal.
    // Discard them while still in raw mode, before the shell can consume them.
    #[cfg(unix)]
    unsafe {
        libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use rstop::app::SignalTarget;

    fn target(child: &std::process::Child) -> SignalTarget {
        let mut system = sysinfo::System::new();
        let pid = sysinfo::Pid::from_u32(child.id());
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        SignalTarget {
            pid: child.id(),
            start_time: system.process(pid).unwrap().start_time(),
            name: "rstop-test-child".into(),
            signal: sysinfo::Signal::Term,
        }
    }

    #[test]
    fn signal_refuses_changed_identity_and_protected_pids() {
        let mut child = std::process::Command::new("sleep")
            .arg("20")
            .spawn()
            .unwrap();
        let mut target = target(&child);
        target.start_time = target.start_time.saturating_sub(1);
        assert!(send_signal(&target).is_err());
        assert!(child.try_wait().unwrap().is_none());
        for pid in [0, 1, std::process::id()] {
            target.pid = pid;
            assert!(send_signal(&target).is_err());
        }
        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn pidfd_sends_signal_only_to_owned_test_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("20")
            .spawn()
            .unwrap();
        let target = target(&child);
        let result = send_signal(&target);
        if result.is_err() {
            let _ = child.kill();
        }
        child.wait().unwrap();
        result.unwrap();
    }
}
