use super::Input;
use crate::runtime::control::send_signal;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rstop::app::{App, MenuKind, Sort};
use std::{sync::atomic::Ordering, thread};
impl Input<'_> {
    pub(super) fn modal(&mut self, app: &mut App, mut key: KeyEvent) -> bool {
        if app
            .ui_menu
            .as_ref()
            .is_some_and(|m| m.kind == MenuKind::Ros)
            && self.ros_filter_key(app, key)
        {
            return true;
        }
        let interval = self.interval;
        let detail_tx = &self.detail_tx;
        let action_tx = &self.action_tx;
        let path = self.path;
        if let Some(mut menu) = app.ui_menu.take() {
            if menu.kind == MenuKind::Columns && key.code == KeyCode::Char(' ') {
                key.code = KeyCode::Enter;
            }
            if key.code != KeyCode::Enter
                && let Some(index) = menu
                    .items
                    .iter()
                    .position(|(_, code)| *code != KeyCode::Null && *code == key.code)
            {
                if matches!(menu.kind, MenuKind::TagActions | MenuKind::Ros) {
                    menu.selected = index;
                    key.code = KeyCode::Enter;
                }
                if menu.kind == MenuKind::Actions {
                    app.main_menu = None;
                    self.queued_key = Some(KeyEvent::new(key.code, KeyModifiers::NONE));
                    return true;
                }
                if matches!(
                    menu.kind,
                    MenuKind::TimeWindow
                        | MenuKind::Display
                        | MenuKind::ConnectionSort
                        | MenuKind::InterfaceSort
                        | MenuKind::ProcessSort
                        | MenuKind::WorkloadSort
                ) {
                    menu.selected = index;
                    key.code = KeyCode::Enter;
                }
            }
            match key.code {
                KeyCode::Esc => {
                    if menu.kind == MenuKind::Ros {
                        self.ros_back(app);
                    }
                    if matches!(menu.kind, MenuKind::TagActions | MenuKind::TagFilter) {
                        app.tag_menu_context = None;
                    }
                }
                KeyCode::Up => {
                    menu.selected = menu.selected.saturating_sub(1);
                    app.ui_menu = Some(menu);
                }
                KeyCode::Down => {
                    menu.selected = (menu.selected + 1).min(menu.items.len().saturating_sub(1));
                    app.ui_menu = Some(menu);
                }
                KeyCode::Home => {
                    menu.selected = 0;
                    app.ui_menu = Some(menu);
                }
                KeyCode::End => {
                    menu.selected = menu.items.len().saturating_sub(1);
                    app.ui_menu = Some(menu);
                }
                KeyCode::PageDown => {
                    menu.selected = (menu.selected + 12).min(menu.items.len().saturating_sub(1));
                    app.ui_menu = Some(menu);
                }
                KeyCode::PageUp => {
                    menu.selected = menu.selected.saturating_sub(12);
                    app.ui_menu = Some(menu);
                }
                KeyCode::Enter => {
                    if menu.kind == MenuKind::Ros {
                        self.ros_action(app, menu.selected);
                        return true;
                    }
                    if menu.kind == MenuKind::TagFilter {
                        if let Some(context) = app.tag_menu_context.take()
                            && let Some(tag) = context.tags.get(menu.selected)
                        {
                            app.filter = format!("tag:{}", tag.key);
                            app.object_scope = None;
                            app.selected = 0;
                            app.table_offset = 0;
                            app.clear_process_selection();
                            app.selected_connection = None;
                        }
                        return true;
                    }
                    if menu.kind == MenuKind::TagActions {
                        if let Some(context) = app.tag_menu_context.take()
                            && let Some((_, _, action)) = context.actions().get(menu.selected)
                        {
                            let action = *action;
                            self.label_action(app, context, action);
                        }
                        return true;
                    }
                    let code = menu
                        .items
                        .get(menu.selected)
                        .map(|(_, code)| *code)
                        .unwrap_or(KeyCode::Null);
                    if menu.kind == MenuKind::Display {
                        if code != KeyCode::Null {
                            self.queued_key = Some(KeyEvent::new(code, KeyModifiers::NONE));
                        } else {
                            match app.view {
                                rstop::app::View::Overview if !app.has_process_table() => {
                                    if let Some(panel) =
                                        ["all", "cpu", "memory", "gpu", "storage", "network"]
                                            .get(menu.selected)
                                    {
                                        app.config.display.overview_panel = (*panel).into();
                                    }
                                }
                                rstop::app::View::Network => app.focus_panel(menu.selected),
                                rstop::app::View::Gpu => {
                                    app.focus_panel(if menu.selected == 0 { 0 } else { 2 })
                                }
                                _ if app.has_process_table() => {
                                    app.config.display.group_processes = menu.selected == 2;
                                    app.tree = menu.selected == 1;
                                    app.selected = 0;
                                    app.clear_process_selection();
                                    app.resort_processes();
                                }
                                _ => {}
                            }
                        }
                    } else if matches!(
                        menu.kind,
                        MenuKind::ConnectionSort
                            | MenuKind::InterfaceSort
                            | MenuKind::ProcessSort
                            | MenuKind::WorkloadSort
                    ) && code != KeyCode::Null
                    {
                        self.queued_key = Some(KeyEvent::new(code, KeyModifiers::NONE));
                    } else if menu.kind == MenuKind::TimeWindow {
                        if let Some(seconds) = app.time_windows().get(menu.selected) {
                            app.set_time_window(*seconds);
                        }
                    } else if menu.kind == MenuKind::Columns {
                        if let Some(column) = rstop::fields::PROCESS_COLUMNS.get(menu.selected) {
                            if let Some(index) = app.config.columns.iter().position(|c| c == column)
                            {
                                if app.config.columns.len() > 1 {
                                    app.config.columns.remove(index);
                                }
                            } else {
                                app.config.columns.push(column.to_string());
                            }
                            menu.items = rstop::fields::PROCESS_COLUMNS
                                .iter()
                                .map(|column| {
                                    (
                                        format!(
                                            "{} {column}",
                                            if app.config.columns.iter().any(|c| c == column) {
                                                "[x]"
                                            } else {
                                                "[ ]"
                                            }
                                        ),
                                        KeyCode::Null,
                                    )
                                })
                                .collect();
                            app.column_offset = 0;
                            app.ui_menu = Some(menu);
                        }
                    } else if menu.kind == MenuKind::ProcessSort {
                        if let Some(sort) = menu
                            .items
                            .get(menu.selected)
                            .and_then(|(name, _)| Sort::from_name(name))
                        {
                            app.sort = sort;
                            app.reverse = false;
                            app.resort_processes();
                        }
                    } else if matches!(
                        menu.kind,
                        MenuKind::ConnectionSort | MenuKind::InterfaceSort
                    ) {
                        app.set_network_sort(menu.selected, false);
                    } else if menu.kind == MenuKind::WorkloadSort {
                        app.workload_sort = menu
                            .items
                            .get(menu.selected)
                            .and_then(|(name, _)| Sort::from_name(name));
                        app.reverse = false;
                        app.selected = 0;
                        app.table_offset = 0;
                    } else if let Some((_, code)) = menu.items.get(menu.selected) {
                        app.main_menu = None;
                        self.queued_key = Some(KeyEvent::new(*code, KeyModifiers::NONE));
                    }
                }
                _ => app.ui_menu = Some(menu),
            }
            return true;
        }
        if let Some((items, mut selected)) = app.action_menu.take() {
            if let KeyCode::Char(c) = key.code
                && let Some(index) = items
                    .iter()
                    .position(|(label, _)| rstop::management::action_key(label) == Some(c))
            {
                selected = index;
                key.code = KeyCode::Enter;
            }
            match key.code {
                KeyCode::Up => app.action_menu = Some((items, selected.saturating_sub(1))),
                KeyCode::Down => {
                    let next = (selected + 1).min(items.len().saturating_sub(1));
                    app.action_menu = Some((items, next));
                }
                KeyCode::Esc => {}
                KeyCode::Home => app.action_menu = Some((items, 0)),
                KeyCode::End => {
                    let last = items.len().saturating_sub(1);
                    app.action_menu = Some((items, last));
                }
                KeyCode::PageDown => {
                    selected = (selected + 12).min(items.len().saturating_sub(1));
                    app.action_menu = Some((items, selected));
                }
                KeyCode::PageUp => {
                    app.action_menu = Some((items, selected.saturating_sub(12)));
                }
                KeyCode::Enter => {
                    if let Some((_, plan)) = items.get(selected).cloned() {
                        if !plan.changes_state {
                            app.detail_generation += 1;
                            let generation = app.detail_generation;
                            app.detail = Some((plan.title.clone(), "Loading…".into()));
                            app.detail_pending = true;
                            let sender = detail_tx.clone();
                            thread::spawn(move || {
                                let body = plan.execute(true).unwrap_or_else(|e| e.to_string());
                                let _ = sender.send((generation, plan.title, body));
                            });
                        } else {
                            app.pending_plan = Some(plan);
                            app.editor.reset();
                            app.prompt = Some(("workload action".into(), String::new(), None));
                        }
                    }
                }
                _ => app.action_menu = Some((items, selected)),
            }
            return true;
        }
        if let Some((kind, mut value, target)) = app.prompt.take() {
            if (app.pending_plan.is_some() || kind == "capture confirm")
                && key.code == KeyCode::Char('y')
            {
                app.prompt = Some((kind, "yes".into(), target));
                self.queued_key = Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                return true;
            }
            match key.code {
                KeyCode::Esc => {
                    app.pending_plan = None;
                    app.capture = None;
                }
                KeyCode::Enter => {
                    if kind.starts_with("capture ") {
                        if app.replay || self.read_only {
                            app.capture = None;
                            app.status = "Packet capture disabled".into();
                        } else if let Some(mut capture) = app.capture.take() {
                            let next = match kind.as_str() {
                                "capture interface" => {
                                    capture.interface = value.trim().into();
                                    Some(("capture filter", capture.filter.clone()))
                                }
                                "capture filter" => {
                                    capture.filter = value.trim().into();
                                    Some((
                                        "capture output",
                                        capture.output.to_string_lossy().into_owned(),
                                    ))
                                }
                                "capture output" => {
                                    capture.output = std::path::PathBuf::from(value.trim());
                                    Some(("capture confirm", String::new()))
                                }
                                "capture confirm" if value == "yes" => {
                                    match capture.validate() {
                                        Ok(()) => super::super::inspect::task(
                                            app,
                                            "PACKET CAPTURE",
                                            detail_tx.clone(),
                                            move || capture.run(),
                                        ),
                                        Err(e) => app.status = e.to_string(),
                                    }
                                    return true;
                                }
                                _ => None,
                            };
                            if let Some((next, value)) = next {
                                app.editor.reset();
                                app.prompt = Some((next.into(), value, None));
                                app.capture = Some(capture);
                            }
                        }
                    } else if ["network target", "PCAP file", "folder path"]
                        .contains(&kind.as_str())
                        && !app.replay
                    {
                        let input = value.trim().to_string();
                        super::super::inspect::task(
                            app,
                            &kind.clone(),
                            detail_tx.clone(),
                            move || match kind.as_str() {
                                "network target" => rstop::network_tools::check_target(&input),
                                "PCAP file" => {
                                    rstop::network_tools::read_capture(std::path::Path::new(&input))
                                }
                                _ => {
                                    rstop::diagnostics::folder_report(std::path::Path::new(&input))
                                }
                            },
                        );
                    } else if let Some(plan) = app.pending_plan.take() {
                        if value == "yes" && !app.replay && !self.read_only {
                            app.detail_generation += 1;
                            let generation = app.detail_generation;
                            app.detail =
                                Some((plan.title.clone(), "Executing… · [Esc] Back".into()));
                            app.detail_pending = true;
                            let sender = detail_tx.clone();
                            thread::spawn(move || {
                                let body = plan
                                    .execute(false)
                                    .unwrap_or_else(|e| format!("Action failed: {e}"));
                                let _ = sender.send((generation, plan.title, body));
                            });
                        } else {
                            app.status = "Action canceled or disabled; type yes to confirm".into();
                        }
                    } else if kind == "log file" && !value.trim().is_empty() && !app.replay {
                        app.detail_generation += 1;
                        app.detail_pending = false;
                        let path = std::path::PathBuf::from(value.trim());
                        app.log_target = Some(rstop::logs::Target::File(path));
                        app.follow_logs = true;
                        app.detail_filter.clear();
                        app.detail_column = 0;
                        super::super::inspect::load_logs(app, detail_tx.clone());
                    } else if let Some(process) = target {
                        if app.replay || self.read_only {
                            app.status = "Process actions disabled in replay/read-only mode".into();
                        } else if ["nice", "affinity", "I/O priority", "autogroup nice"]
                            .contains(&kind.as_str())
                        {
                            app.status = match if kind == "I/O priority" {
                                rstop::process_tools::io_priority(&process, &value)
                            } else if kind == "autogroup nice" {
                                value.parse::<i32>().map_err(anyhow::Error::from).and_then(
                                    |value| {
                                        rstop::process_tools::autogroup_priority(&process, value)
                                    },
                                )
                            } else if kind == "nice" {
                                value
                                    .parse::<i32>()
                                    .map_err(anyhow::Error::from)
                                    .and_then(|n| rstop::process_tools::priority(&process, n))
                            } else {
                                rstop::process_tools::affinity(&process, &value)
                            } {
                                Ok(message) => message,
                                Err(error) => error.to_string(),
                            };
                        } else if value == "yes" {
                            let sender = detail_tx.clone();
                            app.detail_generation += 1;
                            let generation = app.detail_generation;
                            app.detail_pending = true;
                            app.detail = Some((kind.clone(), "Inspecting… · [Esc] Cancel".into()));
                            thread::spawn(move || {
                                let body = rstop::process_tools::inspect(&process, &kind)
                                    .unwrap_or_else(|e| e.to_string());
                                let _ = sender.send((generation, kind, body));
                            });
                        }
                    }
                }
                _ => {
                    app.editor.edit(&mut value, key);
                    app.prompt = Some((kind, value, target));
                }
            }
            return true;
        }
        if let Some(mut target) = app.signal.take() {
            if matches!(key.code, KeyCode::Up | KeyCode::Down) {
                let signals = sysinfo::SUPPORTED_SIGNALS;
                let current = signals
                    .iter()
                    .position(|s| *s == target.signal)
                    .unwrap_or(0);
                let next = if key.code == KeyCode::Up {
                    (current + signals.len() - 1) % signals.len()
                } else {
                    (current + 1) % signals.len()
                };
                target.signal = signals[next];
                app.signal = Some(target);
            } else if key.code == KeyCode::Char('y') {
                let mut targets = std::mem::take(&mut app.signal_targets);
                if targets.is_empty() {
                    targets.push(target.clone());
                }
                let sender = action_tx.clone();
                app.status = format!(
                    "Sending {:?} to {} processes…",
                    target.signal,
                    targets.len()
                );
                thread::spawn(move || {
                    let mut succeeded = 0;
                    let mut failures = vec![];
                    for item in &mut targets {
                        item.signal = target.signal;
                        match send_signal(item) {
                            Ok(()) => succeeded += 1,
                            Err(error) => failures.push(format!("PID {}: {error}", item.pid)),
                        }
                    }
                    let _ = sender.send(format!(
                        "Signal: {succeeded} succeeded, {} failed{}",
                        failures.len(),
                        if failures.is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", failures.join("; "))
                        }
                    ));
                });
            } else if key.code == KeyCode::Esc {
                app.signal_targets.clear();
            } else if matches!(key.code, KeyCode::PageDown | KeyCode::PageUp) {
                app.detail_scroll =
                    app.detail_scroll
                        .saturating_add_signed(if key.code == KeyCode::PageDown {
                            15
                        } else {
                            -15
                        });
                app.signal = Some(target);
            }
            return true;
        }
        if app.settings {
            match key.code {
                KeyCode::Esc | KeyCode::F(2) => app.settings = false,
                KeyCode::Up => app.setting_index = app.setting_index.saturating_sub(1),
                KeyCode::Down => {
                    app.setting_index =
                        (app.setting_index + 1).min(rstop::config::Config::SETTINGS_COUNT - 1)
                }
                KeyCode::Home => app.setting_index = 0,
                KeyCode::End => app.setting_index = rstop::config::Config::SETTINGS_COUNT - 1,
                KeyCode::PageUp => app.setting_index = app.setting_index.saturating_sub(8),
                KeyCode::PageDown => {
                    app.setting_index =
                        (app.setting_index + 8).min(rstop::config::Config::SETTINGS_COUNT - 1)
                }
                KeyCode::Left | KeyCode::Right => {
                    let increase = key.code == KeyCode::Right;
                    match app.setting_index {
                        0 => {
                            app.config.cycle_theme(increase);
                        }
                        1 => {
                            app.config.interval_ms = app
                                .config
                                .interval_ms
                                .saturating_add_signed(if increase { 250 } else { -250 })
                                .clamp(250, 10000);
                            interval.store(app.config.interval_ms, Ordering::Relaxed);
                            app.set_time_window(app.config.time_window_seconds);
                        }
                        2 => {
                            app.config.history = app
                                .config
                                .history
                                .saturating_add_signed(if increase { 30 } else { -30 })
                                .clamp(30, 3600);
                            app.graph_window = app.config.history;
                        }
                        3 => app.config.show_loopback = !app.config.show_loopback,
                        4 => {
                            app.config.cpu_warning = (app.config.cpu_warning
                                + if increase { 5.0 } else { -5.0 })
                            .clamp(0.0, 100.0)
                        }
                        5 => {
                            app.config.memory_warning = (app.config.memory_warning
                                + if increase { 5.0 } else { -5.0 })
                            .clamp(0.0, 100.0)
                        }
                        6 => {
                            app.config.gpu_temperature_warning = app
                                .config
                                .gpu_temperature_warning
                                .saturating_add_signed(if increase { 5 } else { -5 })
                                .clamp(30, 120)
                        }
                        7 => {
                            let presets = ["balanced", "compute", "storage", "minimal"];
                            let i = presets
                                .iter()
                                .position(|v| *v == app.config.layout)
                                .unwrap_or(0);
                            app.config.layout =
                                presets[(i + if increase { 1 } else { 3 }) % 4].into();
                            app.config.custom_layout = None;
                        }
                        8 => {
                            app.config.show_threads = !app.config.show_threads;
                            app.resort_processes();
                        }
                        9 => {
                            app.config.show_kernel_threads = !app.config.show_kernel_threads;
                            app.resort_processes();
                        }
                        10 => app.config.ascii = !app.config.ascii,
                        11 => app.config.no_color = !app.config.no_color,
                        12 => {
                            app.config.alert_hold_seconds = app
                                .config
                                .alert_hold_seconds
                                .saturating_add_signed(if increase { 1 } else { -1 })
                                .min(3600)
                        }
                        13 => {
                            app.config.log_lines = app
                                .config
                                .log_lines
                                .saturating_add_signed(if increase { 100 } else { -100 })
                                .clamp(1, 100000)
                        }
                        index if index >= 14 => {
                            let networks = app
                                .snapshot
                                .networks
                                .iter()
                                .map(|n| n.name.clone())
                                .collect::<Vec<_>>();
                            let sensors = app
                                .snapshot
                                .sensors
                                .iter()
                                .map(|s| s.name.clone())
                                .collect::<Vec<_>>();
                            app.config
                                .display
                                .adjust(index - 14, increase, &networks, &sensors);
                            if app.process_grouped() {
                                app.tree = false;
                                app.clear_process_selection();
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('w') => {
                    app.status = match app.config.save(path) {
                        Ok(()) => format!("Saved {}", path.display()),
                        Err(error) => format!("Cannot save: {error}"),
                    }
                }
                _ => {}
            }
            return true;
        }
        if app.detail_search {
            match key.code {
                KeyCode::Esc => {
                    app.detail_filter.clear();
                    app.detail_search = false;
                }
                KeyCode::Enter => app.detail_search = false,
                _ => {
                    app.editor.edit(&mut app.detail_filter, key);
                }
            }
            app.detail_scroll = 0;
            return true;
        }
        if app.detail.is_some() || app.help {
            match key.code {
                KeyCode::Char('/') | KeyCode::F(3) if app.detail.is_some() => {
                    app.editor.reset();
                    app.detail_search = true;
                }
                KeyCode::Home => {
                    app.follow_logs = false;
                    app.detail_scroll = 0;
                }
                KeyCode::End => {
                    app.detail_scroll = app.detail.as_ref().map_or(0, |(_, body)| {
                        let filtered = body
                            .lines()
                            .filter(|line| {
                                app.detail_filter.is_empty()
                                    || line
                                        .to_lowercase()
                                        .contains(&app.detail_filter.to_lowercase())
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let count = if app.log_target.is_some() {
                            filtered.lines().count()
                        } else {
                            ratatui::widgets::Paragraph::new(filtered)
                                .wrap(ratatui::widgets::Wrap { trim: false })
                                .line_count(app.detail_width)
                        };
                        count.saturating_sub(app.detail_rows)
                    });
                    app.follow_logs = app.log_target.is_some();
                }
                KeyCode::Left if app.log_target.is_some() => {
                    app.detail_column = app.detail_column.saturating_sub(8)
                }
                KeyCode::Right if app.log_target.is_some() => {
                    app.detail_column = app.detail_column.saturating_add(8)
                }
                KeyCode::Char('p') if app.log_target.is_some() => {
                    app.editor.reset();
                    app.prompt = Some(("log file".into(), String::new(), None));
                }
                KeyCode::Char('f') if app.log_target.is_some() => {
                    app.follow_logs = !app.follow_logs;
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                    app.follow_logs = false;
                    app.detail_generation += 1;
                    app.detail_pending = false;
                    app.detail = None;
                    app.log_target = None;
                    app.detail_filter.clear();
                    app.detail_search = false;
                    app.help = false;
                    app.detail_scroll = 0;
                    app.detail_column = 0;
                    self.restore_ros(app);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_sub(1)
                }
                KeyCode::PageDown => {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_add(15);
                }
                KeyCode::PageUp => {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_sub(15);
                }
                _ => {}
            }
            return true;
        }

        false
    }
}
