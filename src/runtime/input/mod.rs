mod dialogs;
mod labels;
mod menu;
mod mouse;
#[cfg(test)]
mod popup_tests;
mod ros;
mod signals;
mod updates;
use super::inspect::{
    additional, inspect, inspect_process_extra, load_logs, load_resource_detail, open_report, task,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(test)]
use rstop::app::SignalTarget;
use rstop::app::{App, MenuKind, Sort, UiMenu, View};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const DOUBLE_ESCAPE_WINDOW: Duration = Duration::from_millis(500);

pub(super) struct Input<'a> {
    pub interval: &'a AtomicU64,
    pub detail_tx: mpsc::Sender<(u64, String, String)>,
    pub action_tx: mpsc::Sender<String>,
    pub update_tx: mpsc::Sender<rstop::updates::Event>,
    pub path: &'a PathBuf,
    pub read_only: bool,
    pub replay_index: &'a AtomicUsize,
    pub replay_len: usize,
    pub queued_key: Option<KeyEvent>,
    pub last_click: Option<(Instant, u16, u16)>,
    pub last_escape: Option<Instant>,
    pub ros_job: Option<ros::Job>,
}

impl Input<'_> {
    pub fn terminal_key(&mut self, app: &mut App, key: KeyEvent, now: Instant) -> bool {
        if key.kind == KeyEventKind::Release {
            return true;
        }
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            if key.kind == KeyEventKind::Repeat {
                return true;
            }
            if self
                .last_escape
                .replace(now)
                .is_some_and(|last| now.saturating_duration_since(last) <= DOUBLE_ESCAPE_WINDOW)
            {
                return false;
            }
        } else {
            self.last_escape = None;
        }
        self.key(app, key)
    }

    pub fn key(&mut self, app: &mut App, mut key: KeyEvent) -> bool {
        if app.prompt.is_none() && app.update_notice.is_none() {
            app.dialog.error.clear();
            app.dialog.scroll = 0;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        if key.code == KeyCode::F(10) {
            return false;
        }
        if app.update_notice.is_some() {
            self.update_key(app, key);
            return true;
        }
        // Process reports keep the same action keys as the process table.
        // Editors and child dialogs retain first claim on their own input.
        if app.detail.is_some()
            && !app.detail_search
            && !app.help
            && !app.settings
            && !app.editing_filter
            && app.ui_menu.is_none()
            && app.action_menu.is_none()
            && app.prompt.is_none()
            && app.signal.is_none()
            && app.main_menu.is_none()
            && app.has_process_table()
            && app.selected_process().is_some()
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && matches!(
                key.code,
                KeyCode::F(9) | KeyCode::Char('k' | 'K' | 't' | 's')
            )
        {
            self.open_signal(app, key.code);
            return true;
        }
        if self.modal(app, key) {
            return true;
        }
        if app.main_menu.is_some() {
            return self.main_menu_key(app, key);
        }
        if !app.gpu_info_open()
            && !app.editing_filter
            && key.modifiers == KeyModifiers::ALT
            && matches!(key.code, KeyCode::Char('j' | 'k'))
        {
            app.move_selection(if key.code == KeyCode::Char('j') {
                1
            } else {
                -1
            });
            return true;
        }
        if !app.gpu_info_open()
            && !app.editing_filter
            && key.modifiers.contains(KeyModifiers::ALT)
            && app.process_detail_rect.height > 0
        {
            match key.code {
                KeyCode::Up => app.scroll_selection(-1),
                KeyCode::Down => app.scroll_selection(1),
                KeyCode::PageUp => app.scroll_selection(-(app.selection_viewport.max(1) as isize)),
                KeyCode::PageDown => app.scroll_selection(app.selection_viewport.max(1) as isize),
                KeyCode::Home => app.jump_selection(false),
                KeyCode::End => app.jump_selection(true),
                _ => return true,
            }
            return true;
        }
        let interval = self.interval;
        let detail_tx = &self.detail_tx;
        let path = self.path;
        let replay_index = self.replay_index;
        let replay_len = self.replay_len;
        let alias = match key.code {
            KeyCode::F(3) => Some(KeyCode::Char('/')),
            KeyCode::F(7) => Some(KeyCode::Char('l')),
            _ => None,
        };
        if let Some(code) = alias {
            app.editing_filter = false;
            key.code = code;
        }
        if matches!(key.code, KeyCode::F(4) | KeyCode::F(5) | KeyCode::F(6)) {
            app.editing_filter = false;
        }
        if app.editing_filter {
            match key.code {
                KeyCode::Enter => app.editing_filter = false,
                KeyCode::Esc => {
                    app.filter = app.filter_before_edit.clone();
                    app.editing_filter = false;
                }
                KeyCode::Up | KeyCode::Down => {
                    app.editing_filter = false;
                    app.move_selection(if key.code == KeyCode::Down { 1 } else { -1 });
                    return true;
                }
                _ => {
                    app.editor.edit(&mut app.filter, key);
                }
            }
            app.selected = 0;
            app.table_offset = 0;
            app.clear_active_selection();
            return true;
        }
        if app.gpu_info_open() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q' | 'I' | 'S' | 'G' | 'D') => {
                    app.focus_panel(0);
                    self.last_click = None;
                }
                KeyCode::Char('/' | 'f') => app.begin_filter(),
                KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => app.move_selection(-1),
                KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => app.move_selection(1),
                KeyCode::PageUp => app.move_selection(-(app.table_rect.height.max(1) as isize)),
                KeyCode::PageDown => app.move_selection(app.table_rect.height.max(1) as isize),
                KeyCode::Home | KeyCode::End => {
                    app.selected = if key.code == KeyCode::Home {
                        0
                    } else {
                        app.row_count().saturating_sub(1)
                    };
                    app.remember_selection();
                }
                KeyCode::Delete => {
                    app.filter.clear();
                    app.selected = 0;
                    app.table_offset = 0;
                }
                KeyCode::Enter | KeyCode::Char('d') => inspect(app),
                _ => {}
            }
            return true;
        }
        if app.process_grouped() && matches!(key.code, KeyCode::Enter | KeyCode::Char('d')) {
            app.open_program_group();
            return true;
        }
        if app.has_process_table()
            && app.selected_process.is_none()
            && matches!(
                key.code,
                KeyCode::Enter
                    | KeyCode::F(9)
                    | KeyCode::Char(
                        'd' | 'e'
                            | 'o'
                            | 'l'
                            | 'n'
                            | 'N'
                            | 'a'
                            | 'x'
                            | 'B'
                            | 'L'
                            | 'k'
                            | 'K'
                            | 't'
                            | 's'
                            | 'v'
                            | 'V'
                            | ' '
                    )
            )
        {
            app.status = "Select a process with ↑↓ or click a row".into();
            return true;
        }
        match key.code {
            KeyCode::F(5) => app.ui_menu = Some(app.display_menu()),
            KeyCode::F(6) => {
                app.ui_menu = Some(UiMenu::new(
                    if app.has_process_table() {
                        "Process tools"
                    } else if app.view == View::Network {
                        "Network tools"
                    } else {
                        "Resource tools"
                    },
                    app.tools_commands(),
                    0,
                ))
            }
            KeyCode::Char('Y') => self.open_label_actions(app),
            KeyCode::Char('R') => {
                let body = rstop::diagnostics::report(app);
                open_report(app, "DIAGNOSE THIS RESOURCE", body);
            }
            KeyCode::Char('O') => additional(app, detail_tx.clone()),
            KeyCode::Char('J') => {
                if app.replay {
                    app.status = "Live kernel messages disabled in replay".into();
                } else {
                    task(app, "KERNEL / OOM", detail_tx.clone(), || {
                        Ok(rstop::diagnostics::kernel_messages())
                    });
                }
            }
            KeyCode::Char('N') if app.view == View::Network => {
                if app.replay {
                    app.status = "Live network checks disabled in replay".into();
                } else {
                    app.editor.reset();
                    let target = app
                        .filtered_connections()
                        .get(app.selected)
                        .filter(|c| {
                            c.state != "LISTEN"
                                && c.state != "BOUND"
                                && c.remote != "0.0.0.0:0"
                                && c.remote != "[::]:0"
                        })
                        .map(|c| c.remote.clone())
                        .unwrap_or_default();
                    app.prompt = Some(("network target".into(), target, None));
                }
            }
            KeyCode::Char('C') if app.view == View::Network => {
                if app.replay || self.read_only {
                    app.status = "Packet capture disabled in replay/read-only mode".into();
                } else {
                    let mut capture = rstop::network_tools::Capture::default();
                    if app.network_section == 1
                        && let Some(n) = app.filtered_networks().get(app.selected)
                    {
                        capture.interface = n.name.clone();
                    }
                    app.editor.reset();
                    app.prompt =
                        Some(("capture interface".into(), capture.interface.clone(), None));
                    app.capture = Some(capture);
                }
            }
            KeyCode::Char('F') if app.view == View::Network => {
                if app.replay {
                    app.status = "Live file inspection disabled in replay".into();
                } else {
                    app.editor.reset();
                    app.prompt = Some(("PCAP file".into(), String::new(), None));
                }
            }
            KeyCode::Char('U') if app.view == View::Storage => {
                if app.replay {
                    app.status = "Live folder scans disabled in replay".into();
                } else {
                    app.editor.reset();
                    app.prompt = Some((
                        "folder path".into(),
                        app.snapshot
                            .disks
                            .get(app.selected)
                            .map(|d| d.mount.clone())
                            .unwrap_or_default(),
                        None,
                    ));
                }
            }
            KeyCode::Char('D') if app.view == View::Network => {
                app.focus_panel(app.network_section + 1)
            }
            KeyCode::F(4) | KeyCode::Char('s') if app.view == View::Network => {
                let (items, index, kind) = if app.network_section == 0 {
                    (
                        rstop::network_sort::ConnectionSort::ALL
                            .iter()
                            .map(|sort| (sort.name().into(), KeyCode::Null))
                            .collect(),
                        app.connection_sort,
                        MenuKind::ConnectionSort,
                    )
                } else {
                    (
                        rstop::network_sort::InterfaceSort::ALL
                            .iter()
                            .map(|sort| (sort.name().into(), KeyCode::Null))
                            .collect(),
                        app.interface_sort,
                        MenuKind::InterfaceSort,
                    )
                };
                app.ui_menu = Some(
                    UiMenu::new(
                        if app.network_section == 0 {
                            "Connection sorting · unknown values last"
                        } else {
                            "Interface sorting · unknown values last"
                        },
                        items,
                        index,
                    )
                    .with_kind(kind)
                    .with_reverse(),
                );
            }
            KeyCode::Char('r') if app.view == View::Network => {
                let index = if app.network_section == 0 {
                    app.connection_sort
                } else {
                    app.interface_sort
                };
                app.set_network_sort(index, true);
            }
            KeyCode::Char('p') if app.view == View::Network && app.network_section == 0 => {
                let pid = app.filtered_connections().get(app.selected).and_then(|c| {
                    c.pids
                        .iter()
                        .find(|pid| {
                            app.snapshot.processes.iter().any(|p| {
                                p.pid == **pid
                                    && p.start_time <= app.snapshot.connections.owners_sampled_at
                            })
                        })
                        .copied()
                });
                if let Some(pid) = pid {
                    app.set_view(View::Processes);
                    app.filter = format!("pid:{pid}");
                    app.selected = 0;
                    app.remember_selection();
                } else {
                    app.status="Owner unavailable: process exited, access restricted, or owner scan is still pending".into();
                }
            }
            KeyCode::Char('m') if app.view == View::Network => {
                app.ui_menu = Some(UiMenu::new("Network tools", app.network_commands(), 0))
            }
            KeyCode::F(2) => app.settings = true,
            KeyCode::Char('q') => return false,
            KeyCode::Char('?') => app.main_menu = Some(Default::default()),
            KeyCode::Char(':') => {
                app.ui_menu = Some(UiMenu::new("Commands", app.commands(), 0));
            }
            KeyCode::F(1) => {
                app.help = true;
                app.detail_scroll = 0;
            }
            KeyCode::Left | KeyCode::Right
                if key
                    .modifiers
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL) =>
            {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.column_offset = app
                        .column_offset
                        .saturating_add_signed(if key.code == KeyCode::Right { 1 } else { -1 })
                        .min(app.visible_columns.len().saturating_sub(1));
                } else if !app.tree {
                    app.horizontal_scroll = app
                        .horizontal_scroll
                        .saturating_add_signed(if key.code == KeyCode::Right { 8 } else { -8 });
                } else if let Some(pid) = app.selected_process().map(|p| p.pid) {
                    if key.code == KeyCode::Left {
                        app.collapsed.insert(pid);
                    } else {
                        app.collapsed.remove(&pid);
                    }
                }
            }
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                let views = app.views();
                let index = views.iter().position(|v| *v == app.view).unwrap_or(0);
                let next = if matches!(key.code, KeyCode::BackTab | KeyCode::Left) {
                    (index + views.len() - 1) % views.len()
                } else {
                    (index + 1) % views.len()
                };
                app.set_view(views[next]);
            }
            KeyCode::Char(c @ '1'..='8') => {
                if let Some(view) = app
                    .views()
                    .into_iter()
                    .find(|v| v.key() == KeyCode::Char(c))
                {
                    app.set_view(view);
                }
            }

            KeyCode::Char('H') => {
                app.config.show_threads = !app.config.show_threads;
                app.resort_processes();
            }
            KeyCode::Char('h') => {
                app.config.show_kernel_threads = !app.config.show_kernel_threads;
                app.resort_processes();
            }
            KeyCode::Char('v' | ' ') if app.has_process_table() => app.toggle_tag(),
            KeyCode::Char('V') if app.has_process_table() => app.tag_children(),
            KeyCode::Char('U') => app.tagged.clear(),
            KeyCode::Char('I') | KeyCode::Char('A') if app.view == View::Processes => {
                if app.replay || self.read_only {
                    app.status = "Process actions disabled in replay/read-only mode".into();
                } else if let Some(process) = app.selected_process().cloned() {
                    app.editor.reset();
                    app.prompt = Some((
                        if key.code == KeyCode::Char('I') {
                            "I/O priority"
                        } else {
                            "autogroup nice"
                        }
                        .into(),
                        String::new(),
                        Some(process),
                    ));
                }
            }
            KeyCode::Char('n' | 'N' | 'a') if app.has_process_table() => {
                if app.replay || self.read_only {
                    app.status = "Process actions disabled in replay/read-only mode".into();
                } else if let Some(p) = app.selected_process().cloned() {
                    app.editor.reset();
                    app.prompt = Some((
                        if matches!(key.code, KeyCode::Char('n' | 'N')) {
                            "nice"
                        } else {
                            "affinity"
                        }
                        .into(),
                        String::new(),
                        Some(p),
                    ));
                }
            }
            KeyCode::Char('x') | KeyCode::Char('B') | KeyCode::Char('L')
                if app.has_process_table() =>
            {
                if app.replay {
                    app.status = "Live inspection disabled during replay".into();
                    return true;
                }
                if let Some(p) = app.selected_process().cloned() {
                    let kind = match key.code {
                        KeyCode::Char('x') => "trace",
                        KeyCode::Char('B') => "stack",
                        _ => "locks",
                    };
                    if kind == "locks" {
                        app.detail = Some((
                            kind.into(),
                            rstop::process_tools::inspect(&p, kind)
                                .unwrap_or_else(|e| e.to_string()),
                        ));
                    } else {
                        app.editor.reset();
                        app.prompt = Some((kind.into(), String::new(), Some(p)));
                    }
                }
            }
            KeyCode::Char(',') | KeyCode::Char('.') if app.replay => {
                let current = replay_index.load(Ordering::Relaxed);
                replay_index.store(
                    current
                        .saturating_add_signed(if key.code == KeyCode::Char(',') {
                            -10
                        } else {
                            10
                        })
                        .min(replay_len.saturating_sub(1)),
                    Ordering::Relaxed,
                );
                app.history = Default::default();
                app.events = Default::default();
            }
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Up => app.move_selection(-1),
            KeyCode::Char('k') if !app.has_process_table() => app.move_selection(-1),
            KeyCode::PageDown => app.move_selection(app.table_rect.height.max(5) as isize),
            KeyCode::PageUp => app.move_selection(-(app.table_rect.height.max(5) as isize)),
            KeyCode::Home => {
                app.selected = 0;
                app.remember_selection();
            }
            KeyCode::End => {
                app.selected = app.row_count().saturating_sub(1);
                app.remember_selection();
            }
            KeyCode::Char('/') | KeyCode::Char('f') => app.begin_filter(),
            KeyCode::Delete if app.can_filter() => {
                app.filter.clear();
                app.selected = 0;
                app.table_offset = 0;
                app.clear_active_selection();
            }
            KeyCode::Esc if app.selection_open => app.close_selection(),
            KeyCode::Esc if app.has_row_selection() => {
                app.clear_active_selection();
            }
            KeyCode::Esc if app.filter.is_empty() => {
                app.main_menu = Some(Default::default());
            }
            KeyCode::Esc => {
                app.filter.clear();
                app.selected = 0;
                app.table_offset = 0;
                app.clear_active_selection();
            }
            KeyCode::F(4) | KeyCode::Char('s')
                if matches!(app.view, View::Services | View::Containers) =>
            {
                let mut items = vec![("Default (discovery order)".into(), KeyCode::Null)];
                items.extend(
                    [Sort::Name, Sort::Cpu, Sort::Memory, Sort::State]
                        .iter()
                        .map(|s| (s.name().into(), KeyCode::Null)),
                );
                app.ui_menu = Some(
                    UiMenu::new("Workload sorting", items, 0)
                        .with_kind(MenuKind::WorkloadSort)
                        .with_reverse(),
                );
            }
            KeyCode::F(4) if app.process_grouped() => {
                app.ui_menu = Some(
                    UiMenu::new(
                        "Sort by",
                        [
                            Sort::Cpu,
                            Sort::Memory,
                            Sort::Name,
                            Sort::Read,
                            Sort::Write,
                            Sort::GpuMemory,
                        ]
                        .into_iter()
                        .map(|sort| (sort.name().into(), KeyCode::Null))
                        .collect(),
                        0,
                    )
                    .with_kind(MenuKind::ProcessSort)
                    .with_reverse(),
                );
            }
            KeyCode::F(4) if app.has_process_table() => {
                let mut sorts = Sort::ALL.to_vec();
                for column in &app.visible_columns {
                    if let Some(sort) = Sort::for_column(column)
                        && !sorts.contains(&sort)
                    {
                        sorts.push(sort);
                    }
                }
                app.ui_menu = Some(
                    UiMenu::new(
                        "Sort by",
                        sorts
                            .iter()
                            .map(|s| (s.name().into(), KeyCode::Null))
                            .collect(),
                        sorts.iter().position(|s| *s == app.sort).unwrap_or(0),
                    )
                    .with_kind(MenuKind::ProcessSort)
                    .with_reverse(),
                );
            }
            KeyCode::Char('C') if app.has_process_table() => {
                if app.config.columns.is_empty() {
                    app.config.columns = app.visible_columns.clone();
                }
                app.ui_menu = Some(
                    UiMenu::new(
                        "Process columns",
                        rstop::fields::PROCESS_COLUMNS
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
                            .collect(),
                        0,
                    )
                    .with_kind(MenuKind::Columns),
                );
            }
            KeyCode::Char('S') | KeyCode::Char('I') if app.view == View::Gpu => {
                app.focus_panel(match key.code {
                    KeyCode::Char('S') => 0,
                    _ => 2,
                });
                self.last_click = None;
            }
            KeyCode::Char('P') | KeyCode::Char('M')
                if app.has_process_table()
                    || matches!(app.view, View::Services | View::Containers) =>
            {
                let sort = if key.code == KeyCode::Char('P') {
                    Sort::Cpu
                } else {
                    Sort::Memory
                };
                if app.has_process_table() {
                    app.sort = sort;
                } else {
                    app.workload_sort = Some(sort);
                }
                app.reverse = false;
                app.selected = 0;
                app.table_offset = 0;
                app.resort_processes();
            }
            KeyCode::Char('S') if app.has_process_table() => {
                app.sort = app.sort.next();
                app.resort_processes();
            }
            KeyCode::Char('r') => {
                app.reverse = !app.reverse;
                app.resort_processes();
            }
            KeyCode::Char('T') => {
                app.tree = !app.tree;
                app.resort_processes();
            }
            KeyCode::Char('g') => {
                app.gpu_only = !app.gpu_only;
                app.resort_processes();
            }
            KeyCode::Char('u') => {
                app.own_only = !app.own_only;
                app.resort_processes();
            }
            KeyCode::Char('c') => {
                app.config.cycle_theme(true);
            }
            KeyCode::Char('i') => app.config.show_loopback = !app.config.show_loopback,
            KeyCode::Char('-') => {
                app.config.interval_ms = app.config.interval_ms.saturating_sub(250).max(250);
                interval.store(app.config.interval_ms, Ordering::Relaxed);
                app.set_time_window(app.config.time_window_seconds);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                app.config.interval_ms = (app.config.interval_ms + 250).min(10000);
                interval.store(app.config.interval_ms, Ordering::Relaxed);
                app.set_time_window(app.config.time_window_seconds);
            }
            KeyCode::Char('W') => {
                let presets = app.time_windows();
                let selected = presets
                    .iter()
                    .position(|s| *s == app.config.time_window_seconds)
                    .unwrap_or(0);
                let items = presets
                    .into_iter()
                    .enumerate()
                    .map(|(index, seconds)| {
                        (
                            if seconds == 0 {
                                "All cached samples".into()
                            } else {
                                format!(
                                    "{} elapsed time",
                                    rstop::app::commands::time_label(seconds)
                                )
                            },
                            KeyCode::Char(if seconds == 0 {
                                '0'
                            } else {
                                (b'1' + index as u8) as char
                            }),
                        )
                    })
                    .collect();
                app.ui_menu = Some(
                    UiMenu::new(
                        &format!(
                            "Time window · cache {}",
                            match app
                                .history
                                .timestamps
                                .back()
                                .zip(app.history.timestamps.front())
                                .map_or(0, |(b, a)| (b - a) as u64)
                            {
                                0 => "0s".into(),
                                seconds => rstop::app::commands::time_label(seconds),
                            }
                        ),
                        items,
                        selected,
                    )
                    .with_kind(MenuKind::TimeWindow),
                );
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => {
                let seconds = if app.config.time_window_seconds == 0 {
                    app.config.history as u64 * app.config.interval_ms / 1000
                } else {
                    app.config.time_window_seconds
                };
                let limit = 3600 * app.config.interval_ms / 1000;
                app.set_time_window(if key.code == KeyCode::Char('z') {
                    (seconds / 2).max(5)
                } else {
                    (seconds * 2).min(limit)
                });
            }
            KeyCode::Char('[') | KeyCode::Char(']') => {
                let count = app.snapshot.gpus.len();
                if count > 0 {
                    app.selected_gpu = if key.code == KeyCode::Char('[') {
                        (app.selected_gpu + count - 1) % count
                    } else {
                        (app.selected_gpu + 1) % count
                    };
                    app.selected = 0;
                    app.clear_process_selection();
                }
            }
            KeyCode::Char('w') => {
                app.config.view = [
                    View::Overview,
                    View::Processes,
                    View::Gpu,
                    View::Storage,
                    View::Network,
                    View::System,
                    View::Containers,
                    View::Services,
                    View::Explorer,
                    View::History,
                    View::Events,
                ]
                .iter()
                .position(|v| *v == app.view)
                .unwrap_or(0);
                app.status = match app.config.save(path) {
                    Ok(()) => format!("Saved {}", path.display()),
                    Err(error) => format!("Cannot save config: {error}"),
                };
            }
            KeyCode::Char('m') if app.has_process_table() => {
                let items = app.process_commands();
                app.ui_menu = Some(UiMenu::new("Advanced process tools", items, 0));
            }
            KeyCode::Char('m') if matches!(app.view, View::Services | View::Containers) => {
                if app.replay {
                    app.status = "Workload management disabled in replay".into();
                } else {
                    let items = if app.view == View::Services {
                        app.filtered_services()
                            .get(app.selected)
                            .map(|s| {
                                rstop::management::service_actions()
                                    .iter()
                                    .filter_map(|action| {
                                        rstop::management::service_plan(s, action)
                                            .ok()
                                            .filter(|plan| !self.read_only || !plan.changes_state)
                                            .map(|plan| (action.to_string(), plan))
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    } else {
                        app.filtered_containers()
                            .get(app.selected)
                            .map(|c| {
                                rstop::management::container_actions()
                                    .iter()
                                    .filter_map(|action| {
                                        rstop::management::container_plan(c, action)
                                            .ok()
                                            .filter(|plan| !self.read_only || !plan.changes_state)
                                            .map(|plan| (action.to_string(), plan))
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    };
                    if !items.is_empty() {
                        app.action_menu = Some((items, 0));
                    }
                }
            }
            KeyCode::Char('m') => {
                app.ui_menu = Some(UiMenu::new("Resource tools", app.tools_commands(), 0))
            }
            KeyCode::Char('G') if app.view == View::Gpu => app.focus_panel(2),
            KeyCode::Char('D') if app.view == View::Gpu => {
                app.focus_panel(if app.gpu_section == 2 { 0 } else { 2 })
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                if key.code == KeyCode::Enter {
                    if app.selection_open {
                        app.close_selection();
                        return true;
                    }
                    if app.open_selection() {
                        return true;
                    }
                }
                inspect(app);
                load_resource_detail(app, detail_tx.clone());
            }
            KeyCode::Char('e') => inspect_process_extra(app, true, detail_tx.clone()),
            KeyCode::Char('o') => inspect_process_extra(app, false, detail_tx.clone()),
            KeyCode::Char('l') => load_logs(app, detail_tx.clone()),
            KeyCode::F(9) | KeyCode::Char('k' | 'K' | 't' | 's')
                if app.has_process_table()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.open_signal(app, key.code);
            }
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstop::{
        config::Config,
        model::{Gpu, Process, Service, Snapshot},
    };
    pub(super) fn run(test: impl FnOnce(&mut Input<'_>, &mut App)) {
        run_at(&PathBuf::from("/unused-rstop-test-config"), test);
    }
    pub(super) fn run_at(path: &PathBuf, test: impl FnOnce(&mut Input<'_>, &mut App)) {
        let interval = AtomicU64::new(1000);
        let replay = AtomicUsize::new(0);
        let (detail_tx, _) = mpsc::channel();
        let (action_tx, _) = mpsc::channel();
        let (update_tx, _) = mpsc::channel();
        let mut input = Input {
            interval: &interval,
            detail_tx,
            action_tx,
            update_tx,
            path,
            read_only: false,
            replay_index: &replay,
            replay_len: 0,
            queued_key: None,
            last_click: None,
            last_escape: None,
            ros_job: None,
        };
        let mut app = App::new(Config::default());
        app.ingest(Snapshot {
            processes: vec![Process {
                pid: 42,
                name: "worker".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        app.set_view(View::Processes);
        test(&mut input, &mut app);
    }
    fn update_release() -> rstop::updates::Release {
        rstop::updates::Release {
            tag_name: "v99.0.0".into(),
            html_url: "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0".into(),
            ..Default::default()
        }
    }

    #[test]
    fn double_escape_quits_across_views_editors_and_nested_dialogs() {
        for view in [
            View::Overview,
            View::Processes,
            View::Gpu,
            View::Storage,
            View::Network,
            View::System,
            View::Containers,
            View::Services,
        ] {
            for context in [
                "page", "filter", "menu", "settings", "help", "detail", "search", "prompt",
                "signal", "commands", "actions", "update",
            ] {
                run(|input, app| {
                    app.set_view(view);
                    match context {
                        "filter" => app.begin_filter(),
                        "menu" => app.main_menu = Some(Default::default()),
                        "settings" => app.settings = true,
                        "help" => app.help = true,
                        "detail" | "search" => {
                            app.detail = Some(("Report".into(), "Contents".into()));
                            app.detail_search = context == "search";
                        }
                        "prompt" => app.prompt = Some(("nice".into(), "0".into(), None)),
                        "signal" => {
                            app.signal = Some(SignalTarget {
                                pid: 42,
                                start_time: 0,
                                name: "worker".into(),
                                signal: sysinfo::Signal::Term,
                            })
                        }
                        "commands" => {
                            app.ui_menu = Some(UiMenu::new("Commands", app.commands(), 0))
                        }
                        "actions" => app.action_menu = Some((vec![], 0)),
                        "update" => {
                            let mut notice = rstop::updates::Notice::new(update_release());
                            notice.phase = rstop::updates::Phase::Installing;
                            app.update_notice = Some(notice);
                        }
                        _ => {}
                    }
                    let now = Instant::now();
                    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
                    assert!(
                        input.terminal_key(app, escape, now),
                        "{view:?}/{context}: first Esc exited"
                    );
                    assert!(
                        !input.terminal_key(app, escape, now + Duration::from_millis(100)),
                        "{view:?}/{context}: second Esc did not exit"
                    );
                });
            }
        }
    }

    #[test]
    fn double_escape_requires_consecutive_unmodified_presses_within_the_window() {
        run(|input, app| {
            let now = Instant::now();
            let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert!(input.terminal_key(app, escape, now));
            assert!(app.main_menu.is_some());
            assert!(input.terminal_key(app, escape, now + Duration::from_millis(501)));
            assert!(app.main_menu.is_none());
            assert!(!input.terminal_key(app, escape, now + Duration::from_millis(600)));
        });
        for interrupt in [
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::ALT),
        ] {
            run(|input, app| {
                let now = Instant::now();
                let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
                assert!(input.terminal_key(app, escape, now));
                assert!(input.terminal_key(app, interrupt, now + Duration::from_millis(50)));
                assert!(input.terminal_key(app, escape, now + Duration::from_millis(100)));
                assert!(!input.terminal_key(app, escape, now + Duration::from_millis(200)));
            });
        }
    }

    #[test]
    fn escape_release_and_repeat_events_do_not_count_as_a_second_press() {
        run(|input, app| {
            let now = Instant::now();
            let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert!(input.terminal_key(app, escape, now));
            for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
                let key = KeyEvent::new_with_kind(KeyCode::Esc, KeyModifiers::NONE, kind);
                assert!(input.terminal_key(app, key, now + Duration::from_millis(100)));
                assert!(app.main_menu.is_some());
            }
            assert!(!input.terminal_key(app, escape, now + Duration::from_millis(200)));
        });
    }

    #[test]
    fn mouse_cancel_actions_do_not_arm_double_escape_and_break_the_sequence() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            let now = Instant::now();
            let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert!(input.terminal_key(app, escape, now));
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
            terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert!(app.main_menu.is_none());
            assert!(input.last_escape.is_none());
            assert!(input.terminal_key(app, escape, now + Duration::from_millis(100)));
            assert!(!input.terminal_key(app, escape, now + Duration::from_millis(200)));
        });
    }

    #[test]
    fn quit_controls_dispatch_ctrl_c_even_while_q_is_filter_text() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        for editing in [false, true] {
            run(|input, app| {
                if editing {
                    app.begin_filter();
                    assert!(press(input, app, KeyCode::Char('q')));
                    assert_eq!(app.filter, "q");
                }
                let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                let (rect, key) = *app.footer_actions.last().unwrap();
                assert_eq!(
                    key,
                    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
                );
                let text: String = (rect.x..rect.right())
                    .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                    .collect();
                assert!(text.contains("[Ctrl+C] Quit"), "{text}");
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: rect.x + 1,
                        row: rect.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                let queued = input.queued_key.take().unwrap();
                assert_eq!(queued, key);
                assert!(!input.key(app, queued));
            });
        }
        for code in [KeyCode::Char('q'), KeyCode::F(10)] {
            run(|input, app| assert!(!press(input, app, code)));
        }
    }

    #[test]
    fn update_choices_keep_session_ignore_separate_from_persistent_version_ignore() {
        let directory = std::env::temp_dir().join(format!(
            "rstop-update-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("config.toml");
        run_at(&path, |input, app| {
            app.update_notice = Some(rstop::updates::Notice::new(update_release()));
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
            terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
            assert_eq!(app.update_notice.as_ref().unwrap().selected, 0);
            press(input, app, KeyCode::Char('i'));
            assert!(app.update_notice.is_none());
            assert!(app.config.ignored_update_version.is_none());
            assert!(!path.exists());
            app.update_notice = Some(rstop::updates::Notice::new(update_release()));
            press(input, app, KeyCode::Tab);
            press(input, app, KeyCode::Right);
            press(input, app, KeyCode::Enter);
            assert!(app.update_notice.is_none());
            assert_eq!(
                Config::load(&path)
                    .unwrap()
                    .ignored_update_version
                    .as_deref(),
                Some("99.0.0")
            );
            app.update_notice = Some(rstop::updates::Notice::new(update_release()));
            input.read_only = true;
            press(input, app, KeyCode::Char('u'));
            assert!(matches!(
                app.update_notice.as_ref().unwrap().phase,
                rstop::updates::Phase::Failed(_)
            ));
        });
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn update_popup_buttons_fit_and_mouse_ignore_does_not_click_through() {
        run(|input, app| {
            for width in [60, 140] {
                app.update_notice = Some(rstop::updates::Notice::new(update_release()));
                let mut terminal =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                assert_eq!(app.update_buttons.len(), 4);
                for (rect, _) in &app.update_buttons {
                    assert!(!rect.is_empty() && app.popup_rect.contains((rect.x, rect.y).into()));
                }
                let (rect, _) = app
                    .update_buttons
                    .iter()
                    .find(|(_, index)| *index == 1)
                    .unwrap();
                input.mouse(
                    app,
                    crossterm::event::MouseEvent {
                        kind: crossterm::event::MouseEventKind::Down(
                            crossterm::event::MouseButton::Left,
                        ),
                        column: rect.x + rect.width / 2,
                        row: rect.y + rect.height / 2,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                let key = input.queued_key.take().unwrap();
                input.key(app, key);
                assert!(app.update_notice.is_none());
                assert_eq!(app.view, View::Processes);
            }
        });
    }
    pub(super) fn press(input: &mut Input<'_>, app: &mut App, code: KeyCode) -> bool {
        let alive = input.key(app, KeyEvent::new(code, KeyModifiers::NONE));
        while let Some(key) = input.queued_key.take() {
            assert!(input.key(app, key));
        }
        alive
    }

    #[test]
    fn header_sampling_buttons_match_their_numbers_and_work_across_views_and_sizes() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};

        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            run(|input, app| {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                for view in app.views() {
                    app.set_view(view);
                    for replay in [false, true] {
                        app.replay = replay;
                        terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                        let header = (0..width)
                            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
                            .collect::<String>();
                        assert!(!header.contains("LIVE") && !header.contains("Window"));
                        assert_eq!(header.contains("REPLAY"), replay);
                        assert!(header.contains("[-] 1.00s [+]"), "{header}");
                        for (symbol, expected) in [('-', 750), ('+', 1000)] {
                            let rect = app
                                .controls
                                .iter()
                                .find(|(rect, key)| rect.y == 0 && *key == KeyCode::Char(symbol))
                                .unwrap()
                                .0;
                            let label = (rect.x..rect.right())
                                .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                                .collect::<String>();
                            assert_eq!(label, format!("[{symbol}]"));
                            input.mouse(
                                app,
                                MouseEvent {
                                    kind: MouseEventKind::Down(MouseButton::Left),
                                    column: rect.x + 1,
                                    row: rect.y,
                                    modifiers: KeyModifiers::NONE,
                                },
                            );
                            let key = input.queued_key.take().unwrap();
                            assert_eq!(key.code, KeyCode::Char(symbol));
                            input.key(app, key);
                            assert_eq!(app.config.interval_ms, expected);
                            assert_eq!(input.interval.load(Ordering::Relaxed), expected);
                        }
                    }
                }
                app.set_time_window(3600);
                for _ in 0..5 {
                    press(input, app, KeyCode::Char('-'));
                }
                assert_eq!(app.config.interval_ms, 250);
                assert_eq!(app.config.time_window_seconds, 900);
                for _ in 0..45 {
                    press(input, app, KeyCode::Char('+'));
                }
                assert_eq!(app.config.interval_ms, 10000);
                assert_eq!(input.interval.load(Ordering::Relaxed), 10000);
                press(input, app, KeyCode::Char('='));
                assert_eq!(app.config.interval_ms, 10000);
                app.set_view(View::Processes);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                press(input, app, KeyCode::F(2));
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                let rect = app
                    .controls
                    .iter()
                    .find(|(rect, key)| rect.y == 0 && *key == KeyCode::Char('-'))
                    .unwrap()
                    .0;
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: rect.x + 1,
                        row: rect.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                assert!(!app.settings);
                assert!(input.queued_key.is_none());
                assert_eq!(app.config.interval_ms, 10000);
            });
        }
    }

    #[test]
    fn overview_gpu_buttons_match_visible_labels_and_switch_devices_across_layouts() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use rstop::config::LayoutNode;

        for layout in ["balanced", "compute", "expanded", "custom"] {
            for (width, height) in [(60, 18), (100, 38), (160, 48)] {
                run(|input, app| {
                    app.set_view(View::Overview);
                    app.snapshot.gpus = vec![
                        Gpu {
                            id: 3,
                            name: "First device".into(),
                            ..Default::default()
                        },
                        Gpu {
                            id: 7,
                            name: "Second device".into(),
                            ..Default::default()
                        },
                    ];
                    match layout {
                        "compute" => app.config.layout = layout.into(),
                        "expanded" => app.config.display.overview_panel = "gpu".into(),
                        "custom" => {
                            app.config.custom_layout = Some(LayoutNode::Split {
                                direction: "horizontal".into(),
                                children: vec![
                                    LayoutNode::Panel {
                                        panel: "gpu".into(),
                                    },
                                    LayoutNode::Panel {
                                        panel: "processes".into(),
                                    },
                                ],
                                weights: vec![1, 1],
                            });
                        }
                        _ => {}
                    }
                    let mut terminal =
                        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
                            .unwrap();
                    terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                    if layout == "balanced" && width < 100 {
                        assert!(
                            !app.controls
                                .iter()
                                .any(|(_, key)| matches!(key, KeyCode::Char('[' | ']')))
                        );
                        return;
                    }
                    for (key, label, selected) in
                        [('[', "Prev", 1), (']', "Next", 0), (']', "Next", 1)]
                    {
                        let rect = app
                            .controls
                            .iter()
                            .find(|(_, code)| *code == KeyCode::Char(key))
                            .unwrap()
                            .0;
                        assert!(rect.right() <= width && rect.bottom() <= height);
                        let button: String = (rect.x..rect.right())
                            .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                            .collect();
                        assert!(
                            button.contains(key) && button.contains(label),
                            "{layout} {width}x{height}: {button}"
                        );
                        assert!(!button.contains('/'));
                        input.mouse(
                            app,
                            MouseEvent {
                                kind: MouseEventKind::Down(MouseButton::Left),
                                column: rect.x + rect.width / 2,
                                row: rect.y,
                                modifiers: KeyModifiers::NONE,
                            },
                        );
                        let queued = input.queued_key.take().unwrap();
                        assert_eq!(queued.code, KeyCode::Char(key));
                        input.key(app, queued);
                        assert_eq!(app.selected_gpu, selected);
                        assert_eq!(app.view, View::Overview);
                        terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                        let text: String = terminal
                            .backend()
                            .buffer()
                            .content
                            .iter()
                            .map(|cell| cell.symbol())
                            .collect();
                        assert!(text.contains(&app.snapshot.gpus[selected].name));
                        assert!(text.contains(&format!("GPU {}", app.snapshot.gpus[selected].id)));
                        assert!(text.contains(&format!("{}/2", selected + 1)));
                    }
                });
            }
        }
    }

    #[test]
    fn overview_gpu_menu_and_shortcuts_wrap_and_share_selection_with_gpu_page() {
        run(|input, app| {
            app.set_view(View::Overview);
            app.snapshot.gpus = vec![Gpu::default(); 2];
            for (label, key, selected) in [("Previous GPU", '[', 1), ("Next GPU", ']', 0)] {
                assert!(
                    app.commands()
                        .iter()
                        .any(|(name, code)| name == label && *code == KeyCode::Char(key))
                );
                press(input, app, KeyCode::F(5));
                let menu = app.ui_menu.as_mut().unwrap();
                menu.selected = menu
                    .items
                    .iter()
                    .position(|(name, _)| name == label)
                    .unwrap();
                press(input, app, KeyCode::Enter);
                assert_eq!(app.selected_gpu, selected);
                assert_eq!(app.view, View::Overview);
            }
            press(input, app, KeyCode::Char('['));
            assert_eq!(app.selected_gpu, 1);
            app.set_view(View::Gpu);
            assert_eq!(app.selected_gpu, 1);
            press(input, app, KeyCode::Char(']'));
            app.set_view(View::Overview);
            assert_eq!(app.selected_gpu, 0);

            app.config.display.overview_panel = "gpu".into();
            for count in [1, 0] {
                let mut snapshot = app.snapshot.clone();
                snapshot.gpus.truncate(count);
                app.ingest(snapshot);
                let mut terminal =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 18)).unwrap();
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                assert!(
                    !app.controls
                        .iter()
                        .any(|(_, key)| matches!(key, KeyCode::Char('[' | ']')))
                );
                assert!(
                    !app.display_menu()
                        .items
                        .iter()
                        .any(|(_, code)| matches!(code, KeyCode::Char('[' | ']')))
                );
                for key in ['[', ']'] {
                    press(input, app, KeyCode::Char(key));
                    assert_eq!(app.selected_gpu, 0);
                }
            }
        });
    }

    #[test]
    fn arrows_cycle_all_tabs_without_scrolling_or_collapsing_processes() {
        for optional_views in [false, true] {
            for tree in [false, true] {
                run(|input, app| {
                    if optional_views {
                        app.snapshot.gpus.push(Gpu::default());
                        app.snapshot.inventory.runtimes.push("docker".into());
                        app.snapshot
                            .inventory
                            .service_managers
                            .push("systemd".into());
                    }
                    app.tree = tree;
                    let views = app.views();
                    for (index, view) in views.iter().enumerate() {
                        for (key, expected) in [
                            (KeyCode::Right, views[(index + 1) % views.len()]),
                            (
                                KeyCode::Left,
                                views[(index + views.len() - 1) % views.len()],
                            ),
                        ] {
                            app.set_view(*view);
                            if app.has_process_table() {
                                app.move_selection(1);
                            }
                            assert!(press(input, app, key));
                            assert_eq!(app.view, expected);
                            assert_eq!(app.horizontal_scroll, 0);
                            assert_eq!(app.column_offset, 0);
                            assert!(app.collapsed.is_empty());
                        }
                    }
                });
            }
        }
    }
    #[test]
    fn modified_arrows_keep_process_scrolling_tree_and_column_controls() {
        run(|input, app| {
            for (key, expected) in [(KeyCode::Right, 8), (KeyCode::Left, 0)] {
                assert!(input.key(app, KeyEvent::new(key, KeyModifiers::CONTROL)));
                assert_eq!(app.view, View::Processes);
                assert_eq!(app.horizontal_scroll, expected);
            }
            app.tree = true;
            app.move_selection(1);
            input.key(app, KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
            assert!(app.collapsed.contains(&42));
            input.key(app, KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
            assert!(!app.collapsed.contains(&42));
            app.visible_columns = vec!["pid".into(), "name".into()];
            for (key, expected) in [(KeyCode::Right, 1), (KeyCode::Left, 0)] {
                input.key(app, KeyEvent::new(key, KeyModifiers::SHIFT));
                assert_eq!(app.view, View::Processes);
                assert_eq!(app.column_offset, expected);
            }
        });
    }
    #[test]
    fn network_sort_menus_and_clickable_pid_headers_apply_independent_sort_directions() {
        run(|input, app| {
            use rstop::network_sort::{ConnectionSort as C, InterfaceSort as I};
            app.set_view(View::Network);
            press(input, app, KeyCode::F(4));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::ConnectionSort);
            assert_eq!(app.ui_menu.as_ref().unwrap().items.len(), 19);
            app.ui_menu.as_mut().unwrap().selected = C::Upload.index();
            press(input, app, KeyCode::Enter);
            assert_eq!(app.connection_sort, C::Upload.index());
            press(input, app, KeyCode::F(4));
            press(input, app, KeyCode::End);
            press(input, app, KeyCode::Enter);
            assert!(app.connection_reverse);
            assert!(!app.interface_reverse);
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let pid = app
                .network_sort_headers
                .iter()
                .find(|(_, index)| *index == C::Pid.index())
                .unwrap()
                .0;
            input.mouse(
                app,
                crossterm::event::MouseEvent {
                    kind: crossterm::event::MouseEventKind::Down(
                        crossterm::event::MouseButton::Left,
                    ),
                    column: pid.x,
                    row: pid.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.connection_sort, C::Pid.index());
            assert!(!app.connection_reverse);
            input.mouse(
                app,
                crossterm::event::MouseEvent {
                    kind: crossterm::event::MouseEventKind::Down(
                        crossterm::event::MouseButton::Left,
                    ),
                    column: pid.x,
                    row: pid.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert!(app.connection_reverse);
            press(input, app, KeyCode::Char('D'));
            press(input, app, KeyCode::Char('s'));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::InterfaceSort);
            assert_eq!(app.ui_menu.as_ref().unwrap().items.len(), I::ALL.len() + 1);
            app.ui_menu.as_mut().unwrap().selected = I::Errors.index();
            press(input, app, KeyCode::Enter);
            assert_eq!(app.interface_sort, I::Errors.index());
            assert!(app.connection_reverse);
        });
    }
    #[test]
    fn common_view_and_tools_menus_work_across_resources_and_filter_text_is_not_an_action() {
        run(|input, app| {
            app.snapshot.gpus.push(Gpu::default());
            for view in [View::Network, View::Gpu, View::Processes] {
                app.set_view(view);
                press(input, app, KeyCode::F(5));
                assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::Display);
                press(input, app, KeyCode::Down);
                press(input, app, KeyCode::Enter);
                assert!(app.ui_menu.is_none());
                match view {
                    View::Network => assert_eq!(app.network_section, 1),
                    View::Gpu => assert_eq!(app.gpu_section, 2),
                    _ => assert!(app.tree),
                }
            }
            app.set_view(View::Network);
            app.set_network_section(0);
            press(input, app, KeyCode::F(3));
            for code in "NCF".chars() {
                press(input, app, KeyCode::Char(code));
            }
            assert_eq!(app.filter, "NCF");
            assert!(app.prompt.is_none());
            press(input, app, KeyCode::F(6));
            assert!(!app.editing_filter);
            assert_eq!(app.ui_menu.as_ref().unwrap().title, "Network tools");
            press(input, app, KeyCode::Char('C'));
            assert_eq!(app.prompt.as_ref().unwrap().0, "capture interface");
            press(input, app, KeyCode::Esc);
            assert!(app.capture.is_none());
            app.set_view(View::Processes);
            press(input, app, KeyCode::F(6));
            assert!(app.ui_menu.as_ref().unwrap().items.len() <= 9);
            press(input, app, KeyCode::Char('m'));
            assert!(
                app.ui_menu
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .any(|(s, _)| s == "Set CPU affinity")
            );
            press(input, app, KeyCode::Esc);
            app.set_view(View::Services);
            press(input, app, KeyCode::F(6));
            assert!(
                app.ui_menu
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .any(|(s, _)| s == "Manage workload")
            );
            press(input, app, KeyCode::Esc);
            app.set_view(View::Network);
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let region = app.panel_regions.iter().find(|r| r.section == 1).unwrap();
            let rect = region.area;
            let section = region.section;
            input.mouse(
                app,
                crossterm::event::MouseEvent {
                    kind: crossterm::event::MouseEventKind::Down(
                        crossterm::event::MouseButton::Left,
                    ),
                    column: rect.right() - 1,
                    row: rect.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.network_section, section);
        });
    }
    #[test]
    fn clicking_an_inactive_network_header_focuses_and_sorts_only_that_table() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use rstop::model::Network;
        use rstop::network_sort::{ConnectionSort as C, InterfaceSort as I};
        run(|input, app| {
            app.snapshot.networks = vec![Network {
                name: "eth-test".into(),
                ..Default::default()
            }];
            app.set_view(View::Network);
            app.connection_sort = C::Pid.index();
            app.connection_reverse = true;
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let pane = app.panel_regions.iter().find(|r| r.section == 1).unwrap();
            let header = pane
                .network_headers
                .iter()
                .find(|(_, i)| *i == I::Upload.index())
                .unwrap()
                .0;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: header.x,
                    row: header.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.network_section, 1);
            assert_eq!(app.interface_sort, I::Upload.index());
            assert_eq!(app.connection_sort, C::Pid.index());
            assert!(app.connection_reverse);
            let pane = app.panel_regions.iter().find(|r| r.section == 1).unwrap();
            let row = pane.table;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: row.x,
                    row: row.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            press(input, app, KeyCode::Enter);
            assert!(app.detail.as_ref().unwrap().0.contains("eth-test"));
            press(input, app, KeyCode::Esc);
            let row = app
                .panel_regions
                .iter()
                .find(|r| r.section == 0)
                .unwrap()
                .table;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: row.x,
                    row: row.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.network_section, 0);
            assert_eq!(app.connection_sort, C::Pid.index());
            assert!(app.connection_reverse);
        });
    }
    #[test]
    fn network_capture_wizard_only_runs_after_confirmation_and_respects_mode_boundaries() {
        run(|input, app| {
            app.set_view(View::Network);
            input.read_only = true;
            press(input, app, KeyCode::Char('C'));
            assert!(app.prompt.is_none());
            input.read_only = false;
            app.replay = true;
            press(input, app, KeyCode::Char('C'));
            assert!(app.prompt.is_none());
            app.replay = false;
            press(input, app, KeyCode::Char('C'));
            assert_eq!(app.prompt.as_ref().unwrap().0, "capture interface");
            for kind in ["capture filter", "capture output", "capture confirm"] {
                press(input, app, KeyCode::Enter);
                assert_eq!(app.prompt.as_ref().unwrap().0, kind);
            }
            assert!(app.detail.is_none());
            assert!(app.capture.is_some());
            press(input, app, KeyCode::Esc);
            assert!(app.prompt.is_none());
            assert!(app.capture.is_none());
            assert!(!app.detail_pending);
            press(input, app, KeyCode::Char('R'));
            assert!(app.detail.as_ref().unwrap().1.contains("DIAGNOSIS"));
            assert!(app.log_target.is_none());
        });
    }
    #[test]
    fn label_menu_binds_the_selected_object_and_exposes_relevant_keys() {
        run(|input, app| {
            app.ingest(Snapshot {
                processes: vec![Process {
                    pid: 42,
                    start_time: 10,
                    name: "python3".into(),
                    command: "python3 -m http.server 8080".into(),
                    ..Default::default()
                }],
                connections: rstop::connections::Sample {
                    connections: vec![rstop::connections::Connection {
                        id: "owned".into(),
                        pids: vec![42],
                        protocol: "tcp".into(),
                        state: "LISTEN".into(),
                        local: "0.0.0.0:8080".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            });
            app.remember_selection();
            press(input, app, KeyCode::Char('Y'));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TagActions);
            assert!(
                app.ui_menu
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .any(|(_, key)| *key == KeyCode::Char('t'))
            );
            press(input, app, KeyCode::Char('t'));
            assert_eq!(app.prompt.as_ref().unwrap().0, "network target");
            assert_eq!(app.prompt.as_ref().unwrap().1, "127.0.0.1:8080");
            assert!(!app.detail_pending); // A menu choice prepares a target; Enter performs the check.
            press(input, app, KeyCode::Esc);
            press(input, app, KeyCode::Char('Y'));
            app.snapshot.processes[0].name = "other".into(); // Repainting a row must not replace menu evidence.
            press(input, app, KeyCode::Char('e'));
            let detail = app.detail.as_ref().unwrap();
            assert!(detail.0.contains("PID 42"));
            assert!(detail.1.contains("http.server"));
            press(input, app, KeyCode::Esc);
            app.replay = true;
            press(input, app, KeyCode::Char('Y'));
            assert!(
                !app.ui_menu
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .any(|(_, key)| *key == KeyCode::Char('t') || *key == KeyCode::Char('l'))
            );
        });
    }

    #[test]
    fn all_labels_can_be_filtered_and_sidebar_scrolling_does_not_move_the_row() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            app.ingest(Snapshot {
                processes: vec![Process {
                    pid: 42,
                    start_time: 10,
                    name: "python3".into(),
                    command: format!("python3 /jobs/train.py {} END_OF_COMMAND", "x".repeat(2000)),
                    cpu: 130.,
                    memory: 2 * 1024 * 1024 * 1024,
                    ..Default::default()
                }],
                ..Default::default()
            });
            app.remember_selection();
            press(input, app, KeyCode::Enter);
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let identity = app.selected_process;
            input.key(app, KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
            assert_eq!(app.selection_scroll, 1);
            let rect = app.process_detail_rect;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: rect.x + 1,
                    row: rect.y + 1,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.selection_scroll, 4);
            assert_eq!(app.selected_process, identity);
            press(input, app, KeyCode::Char('Y'));
            let tags = app.tag_menu_context.as_ref().unwrap().tags.clone();
            press(input, app, KeyCode::Char('f'));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TagFilter);
            assert_eq!(app.ui_menu.as_ref().unwrap().items.len(), tags.len());
            press(input, app, KeyCode::End);
            press(input, app, KeyCode::Enter);
            assert_eq!(app.filter, format!("tag:{}", tags.last().unwrap().key));
            assert!(app.tag_menu_context.is_none());
        });
    }

    #[test]
    fn label_clicks_select_the_visible_row_and_reused_pids_block_live_inspection() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            app.ingest(Snapshot {
                processes: vec![Process {
                    pid: 42,
                    start_time: 10,
                    name: "custom-robot".into(),
                    command: "custom-robot --ros-args".into(),
                    cpu: 140.,
                    ..Default::default()
                }],
                ..Default::default()
            });
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            // The badge at the table's end is display-only: clicking it selects the row.
            let table = app.table_rect;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: table.right() - 3,
                    row: table.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert_eq!(app.selected_process, Some((42, 10)));
            assert!(input.queued_key.is_none());
            assert!(app.ui_menu.is_none());
            assert!(!app.selection_open);
            press(input, app, KeyCode::Enter);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let hit = app
                .label_targets
                .iter()
                .find(|(_, key)| key == "robotics")
                .unwrap()
                .0;
            assert!(!app.table_rect.contains((hit.x, hit.y).into()));
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: hit.x,
                    row: hit.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            let key = input.queued_key.take().unwrap();
            input.key(app, key);
            assert_eq!(app.selected_process, Some((42, 10)));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TagActions);
            assert_eq!(app.tag_menu_context.as_ref().unwrap().tags.len(), 1);
            assert_eq!(
                app.tag_menu_context.as_ref().unwrap().tags[0].key,
                "robotics"
            );
            assert!(
                !app.ui_menu
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .any(|(_, key)| *key == KeyCode::Char('a'))
            );
            press(input, app, KeyCode::Esc);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let cpu = app
                .label_targets
                .iter()
                .find(|(_, key)| key == "cpu-cores")
                .unwrap()
                .0;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: cpu.x,
                    row: cpu.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            let key = input.queued_key.take().unwrap();
            input.key(app, key);
            assert_eq!(
                app.tag_menu_context.as_ref().unwrap().tags[0].key,
                "cpu-cores"
            );
            if cfg!(target_os = "linux") {
                assert!(
                    app.ui_menu
                        .as_ref()
                        .unwrap()
                        .items
                        .iter()
                        .any(|(_, key)| *key == KeyCode::Char('a'))
                );
                assert!(
                    !app.ui_menu
                        .as_ref()
                        .unwrap()
                        .items
                        .iter()
                        .any(|(_, key)| *key == KeyCode::Char('c'))
                );
            }
            press(input, app, KeyCode::Esc);
            app.selection_scroll = usize::MAX;
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert!(app.label_targets.is_empty());
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: hit.x,
                    row: hit.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert!(input.queued_key.is_none());
            app.selection_scroll = 0;
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let hit = app
                .label_targets
                .iter()
                .find(|(_, key)| key == "robotics")
                .unwrap()
                .0;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: hit.x,
                    row: hit.y,
                    modifiers: KeyModifiers::NONE,
                },
            );
            let key = input.queued_key.take().unwrap();
            input.key(app, key);
            if cfg!(target_os = "linux") {
                app.snapshot.processes[0].start_time = 11;
                press(input, app, KeyCode::Char('r'));
                assert!(app.status.contains("identity changed"));
                assert!(!app.detail_pending);
                assert!(app.detail.is_none());
            }
        });
    }

    #[test]
    fn filters_keep_typed_shortcuts_inside_the_editor_and_cancel_restores_query() {
        run(|input, app| {
            app.filter = "worker".into();
            press(input, app, KeyCode::Char('f'));
            input.key(
                app,
                KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            );
            for c in "中q".chars() {
                assert!(press(input, app, KeyCode::Char(c)));
            }
            press(input, app, KeyCode::Left);
            press(input, app, KeyCode::Char('a'));
            assert_eq!(app.filter, "中aq");
            press(input, app, KeyCode::Esc);
            assert_eq!(app.filter, "worker");
            assert!(!app.editing_filter);
            press(input, app, KeyCode::Char('f'));
            press(input, app, KeyCode::Down);
            assert_eq!(app.filter, "worker");
            assert!(!app.editing_filter);
            app.config.show_threads = true;
            app.snapshot.processes[0].thread = true;
            press(input, app, KeyCode::Char('H'));
            assert!(app.selected_process().is_none());
        });
    }
    #[test]
    fn enter_toggles_detail_panes_and_escape_preserves_selection() {
        use ratatui::{Terminal, backend::TestBackend};
        use rstop::{
            connections::Connection,
            model::{Container, GpuProcess},
        };
        for view in [
            View::Processes,
            View::Gpu,
            View::Network,
            View::Services,
            View::Containers,
        ] {
            run(|input, app| {
                app.snapshot.processes[0].gpu_ids = vec![0];
                app.snapshot.gpus.push(Gpu {
                    processes: vec![GpuProcess {
                        pid: 42,
                        ..Default::default()
                    }],
                    ..Default::default()
                });
                for i in 0..2 {
                    app.snapshot.inventory.services.push(Service {
                        name: format!("worker-{i}.service"),
                        scope: "user".into(),
                        ..Default::default()
                    });
                    app.snapshot.inventory.containers.push(Container {
                        id: format!("container-{i}"),
                        runtime: "docker".into(),
                        ..Default::default()
                    });
                    app.snapshot.connections.connections.push(Connection {
                        id: format!("connection-{i}"),
                        ..Default::default()
                    });
                }
                app.set_view(view);
                let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                press(input, app, KeyCode::Down);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(app.has_row_selection(), "{view:?}");
                assert!(
                    !app.selection_open && app.process_detail_rect.is_empty(),
                    "{view:?}"
                );
                let selected = app.selected;
                press(input, app, KeyCode::Enter);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(
                    app.selection_open && !app.process_detail_rect.is_empty(),
                    "{view:?}"
                );
                assert!(app.detail.is_none(), "{view:?}: pane must not open a popup");
                let snapshot = app.snapshot.clone();
                app.ingest(snapshot);
                assert!(app.selection_open, "{view:?}: refresh closed details");
                press(input, app, KeyCode::Enter);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(
                    !app.selection_open && app.process_detail_rect.is_empty(),
                    "{view:?}: Enter did not close details"
                );
                assert!(app.has_row_selection());
                assert_eq!(app.selected, selected);
                assert!(app.detail.is_none() && app.main_menu.is_none());
                press(input, app, KeyCode::Enter);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(app.selection_open && !app.process_detail_rect.is_empty());
                press(input, app, KeyCode::Esc);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(!app.selection_open && app.process_detail_rect.is_empty());
                assert!(app.has_row_selection());
                assert_eq!(app.selected, selected);
                assert!(app.main_menu.is_none());
                press(input, app, KeyCode::Enter);
                // Refreshing/sorting the same identity preserves the explicit pane.
                app.remember_selection();
                assert!(app.selection_open);
                if matches!(view, View::Network | View::Services | View::Containers) {
                    press(input, app, KeyCode::Up);
                    assert!(!app.selection_open, "{view:?}: a new row opened details");
                    press(input, app, KeyCode::Enter);
                }
                press(input, app, KeyCode::F(3));
                assert!(!app.selection_open);
                press(input, app, KeyCode::Esc);
                assert!(!app.selection_open);
            });
        }
    }

    #[test]
    fn single_click_selects_and_double_click_activates_the_detail_pane() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let click = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: app.table_rect.x + 2,
                row: app.table_rect.y,
                modifiers: KeyModifiers::NONE,
            };
            input.mouse(app, click);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert!(app.has_row_selection());
            assert!(!app.selection_open && app.process_detail_rect.is_empty());
            input.mouse(app, click);
            let key = input.queued_key.take().unwrap();
            assert_eq!(key.code, KeyCode::Enter);
            input.key(app, key);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert!(app.selection_open && !app.process_detail_rect.is_empty());
            assert!(app.detail.is_none());
        });
    }

    #[test]
    fn retired_pause_keys_do_not_stop_live_or_replay_samples() {
        for replay in [false, true] {
            run(|input, app| {
                app.replay = replay;
                for (tick, key) in [KeyCode::F(8), KeyCode::Char(' ')].into_iter().enumerate() {
                    press(input, app, key);
                    let mut sample = app.snapshot.clone();
                    sample.timestamp = 100 + tick as u64;
                    app.ingest(sample);
                    assert_eq!(app.snapshot.timestamp, 100 + tick as u64);
                    assert!(app.detail.is_none() && app.ui_menu.is_none());
                }
            });
        }
    }

    #[test]
    fn mouse_selection_scroll_and_empty_rows_do_not_choose_another_process() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            app.snapshot.processes = (0..40)
                .map(|i| Process {
                    pid: 100 + i,
                    start_time: i as u64,
                    cpu: (40 - i) as f32,
                    ..Default::default()
                })
                .collect();
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert!(app.selected_process().is_none());
            let mut mouse = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: app.table_rect.x + 4,
                row: app.table_rect.y + 2,
                modifiers: KeyModifiers::NONE,
            };
            let pid_x = app.table_rect.x + 1;
            let pid_y = app.table_rect.y + 2;
            let first_digit = terminal.backend().buffer()[(pid_x, pid_y)]
                .symbol()
                .to_owned();
            assert_eq!(first_digit, "1");
            input.mouse(app, mouse);
            assert_eq!(app.selected_process().unwrap().pid, 102);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert_eq!(
                terminal.backend().buffer()[(pid_x, pid_y)].symbol(),
                first_digit
            );
            mouse.kind = MouseEventKind::ScrollDown;
            for _ in 0..4 {
                input.mouse(app, mouse);
            }
            assert_eq!(app.selected_process().unwrap().pid, 102);
            assert!(app.table_offset > 0);
            let expected = 100 + app.table_offset as u32;
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let visible_row = (app.table_rect.x..app.table_rect.right())
                .map(|x| terminal.backend().buffer()[(x, app.table_rect.y)].symbol())
                .collect::<String>();
            assert!(visible_row.contains(&expected.to_string()));
            press(input, app, KeyCode::Esc);
            assert!(app.selected_process().is_none());
            assert!(app.ui_menu.is_none());
            app.filter = "pid:102".into();
            app.table_offset = 0;
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            mouse.kind = MouseEventKind::Down(MouseButton::Left);
            mouse.row = app.table_rect.y;
            input.mouse(app, mouse);
            assert_eq!(app.selected_process().unwrap().pid, 102);
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            mouse.row = app.table_rect.y + 2;
            input.mouse(app, mouse);
            assert!(app.selected_process().is_none());
            let snapshot = app.snapshot.clone();
            app.ingest(snapshot);
            assert!(app.selected_process().is_none());
            press(input, app, KeyCode::F(9));
            assert!(app.signal.is_none());
        });
    }
    #[test]
    fn gpu_info_button_preserves_process_selection_and_supports_search_and_nested_details() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            run(|input, app| {
                app.snapshot.gpus.push(Gpu {
                    uuid: "gpu-0".into(),
                    name: "Test GPU".into(),
                    extra: [("pci_bus_id".into(), serde_json::json!("0000:01:00.0"))].into(),
                    ..Default::default()
                });
                app.snapshot.processes[0].gpu_ids = vec![0];
                app.set_view(View::Gpu);
                app.filter = "worker".into();
                app.move_selection(1);
                app.open_selection();
                let identity = app.selected_process;
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                let button = app
                    .controls
                    .iter()
                    .find(|(_, key)| *key == KeyCode::Char('I'))
                    .unwrap()
                    .0;
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: button.x + 1,
                        row: button.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                press(input, app, KeyCode::Null);
                assert!(app.gpu_info_open());
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                assert!(app.popup_rect.width < width && app.popup_rect.height < height);
                assert!(app.table_rect.height > 1);
                press(input, app, KeyCode::PageDown);
                assert!(app.selected > 0);
                press(input, app, KeyCode::F(3));
                for code in "pci".chars() {
                    press(input, app, KeyCode::Char(code));
                }
                press(input, app, KeyCode::Enter);
                assert_eq!(app.filter, "pci");
                assert_eq!(app.row_count(), 3);
                press(input, app, KeyCode::Char('2'));
                assert_eq!(app.view, View::Gpu);
                press(input, app, KeyCode::Enter);
                assert!(app.detail.as_ref().unwrap().1.contains("0000:01:00.0"));
                press(input, app, KeyCode::Esc);
                assert!(app.detail.is_none() && app.gpu_info_open());
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                let close = app
                    .popup_actions
                    .iter()
                    .find(|(_, key)| *key == KeyCode::Esc)
                    .unwrap()
                    .0;
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: close.x,
                        row: close.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                press(input, app, KeyCode::Null);
                assert!(!app.gpu_info_open());
                assert_eq!(app.filter, "worker");
                assert_eq!(app.selected_process, identity);
                assert!(app.selection_open);
            });
        }
    }

    #[test]
    fn gpu_double_click_opens_the_clicked_device_without_moving_cards_or_clicking_through() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            app.snapshot.gpus = (0..8)
                .map(|id| Gpu {
                    id,
                    uuid: format!("gpu-{id}"),
                    name: format!("Test GPU {id}"),
                    ..Default::default()
                })
                .collect();
            app.set_view(View::Gpu);
            app.selected_gpu = 7;
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
            let cards = app.gpu_ranges.clone();
            let (card, index) = cards[0];
            let click = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: card.x + 4,
                row: card.y + 1,
                modifiers: KeyModifiers::NONE,
            };
            input.mouse(app, click);
            assert_eq!(app.selected_gpu, index);
            assert!(!app.gpu_info_open());
            terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
            assert_eq!(app.gpu_ranges, cards);
            input.mouse(app, click);
            assert!(app.gpu_info_open());
            assert_eq!(app.selected_gpu, index);
            terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
            press(input, app, KeyCode::F(3));
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
            );
            assert!(!app.gpu_info_open() && !app.editing_filter);
            assert_eq!(app.view, View::Gpu);
            assert_eq!(app.selected_gpu, index);
            assert!(app.main_menu.is_none());
            assert!(input.queued_key.is_none());
        });
    }

    #[test]
    fn visible_keys_drive_windows_gpu_sections_sort_and_commands_without_a_mouse() {
        run(|input, app| {
            press(input, app, KeyCode::Char('W'));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TimeWindow);
            press(input, app, KeyCode::Char('3'));
            assert_eq!(app.config.time_window_seconds, 60);
            press(input, app, KeyCode::Char('z'));
            assert_eq!(app.config.time_window_seconds, 30);
            press(input, app, KeyCode::Char('Z'));
            assert_eq!(app.config.time_window_seconds, 60);
            press(input, app, KeyCode::Char('M'));
            assert_eq!(app.sort, Sort::Memory);
            press(input, app, KeyCode::Char('P'));
            assert_eq!(app.sort, Sort::Cpu);
            app.snapshot.gpus.push(Gpu::default());
            app.set_view(View::Gpu);
            for (key, section) in [('I', 2), ('S', 0), ('T', 0)] {
                press(input, app, KeyCode::Char(key));
                assert_eq!(app.gpu_section, section);
            }
            press(input, app, KeyCode::Esc);
            assert!(app.main_menu.is_some());
            press(input, app, KeyCode::Char(':'));
            press(input, app, KeyCode::Char('W'));
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TimeWindow);
        });
    }
    #[test]
    fn outside_clicks_dismiss_popups_without_dispatching_to_the_background() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            for case in [
                "labels",
                "label filter",
                "settings",
                "help",
                "main",
                "view",
                "sort",
                "detail",
                "prompt",
                "plan",
                "capture",
                "signal",
                "actions",
                "searching logs",
            ] {
                run(|input, app| {
                    app.ingest(Snapshot {
                        processes: vec![Process {
                            pid: 42,
                            start_time: 10,
                            name: "nginx".into(),
                            command: "nginx".into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    });
                    app.remember_selection();
                    let identity = app.selected_process;
                    let generation = app.detail_generation;
                    let plan = rstop::management::Plan {
                        title: "TEST ONLY".into(),
                        commands: vec![("/rstop-test-never-execute".into(), vec![])],
                        changes_state: true,
                    };
                    match case {
                        "labels" => {
                            press(input, app, KeyCode::Char('Y'));
                        }
                        "label filter" => {
                            press(input, app, KeyCode::Char('Y'));
                            press(input, app, KeyCode::Char('f'));
                        }
                        "settings" => {
                            press(input, app, KeyCode::F(2));
                            press(input, app, KeyCode::Right);
                        }
                        "help" => {
                            press(input, app, KeyCode::F(1));
                        }
                        "main" => {
                            press(input, app, KeyCode::Char('?'));
                        }
                        "view" => {
                            press(input, app, KeyCode::F(5));
                        }
                        "sort" => {
                            press(input, app, KeyCode::F(4));
                        }
                        "detail" => {
                            app.detail = Some(("sample".into(), "body".into()));
                            app.detail_pending = true;
                        }
                        "prompt" => {
                            app.prompt =
                                Some(("network target".into(), "localhost:1".into(), None));
                        }
                        "plan" => {
                            app.pending_plan = Some(plan);
                            app.prompt = Some(("workload action".into(), "yes".into(), None));
                        }
                        "capture" => {
                            app.capture = Some(Default::default());
                            app.prompt = Some(("capture confirm".into(), "yes".into(), None));
                        }
                        "signal" => {
                            let target = SignalTarget {
                                pid: 42,
                                start_time: 10,
                                name: "TEST ONLY".into(),
                                signal: sysinfo::Signal::Term,
                            };
                            app.signal_targets = vec![target.clone()];
                            app.signal = Some(target);
                        }
                        "actions" => {
                            app.action_menu = Some((vec![("test".into(), plan)], 0));
                        }
                        "searching logs" => {
                            app.detail = Some(("log".into(), "sample body".into()));
                            app.detail_pending = true;
                            app.log_target = Some(rstop::logs::Target::File("/not-read".into()));
                            app.follow_logs = true;
                            app.detail_search = true;
                            app.detail_filter = "sample".into();
                        }
                        _ => unreachable!(),
                    }
                    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                    terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                    let popup = app.popup_rect;
                    assert!(!popup.is_empty(), "{case} {width}x{height}");
                    let mut mouse = MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: popup.x,
                        row: popup.y,
                        modifiers: KeyModifiers::NONE,
                    };
                    input.mouse(app, mouse); // The frame belongs to the popup, too.
                    assert_eq!(app.popup_rect, popup);
                    assert!(
                        input.queued_key.is_none(),
                        "{case}: frame activated an action"
                    );
                    // Clicking empty menu padding must not activate the last action.
                    if let Some(menu) = &app.ui_menu {
                        let blank = menu.rect.y + menu.items.len() as u16;
                        if blank < menu.rect.bottom() {
                            mouse.column = menu.rect.x;
                            mouse.row = blank;
                            input.mouse(app, mouse);
                            assert!(
                                input.queued_key.is_none(),
                                "{case}: empty padding activated an action"
                            );
                        }
                    }
                    mouse.column = 0;
                    mouse.row = 0;
                    input.last_click = Some((Instant::now(), 0, 0));
                    input.mouse(app, mouse);
                    assert!(
                        app.ui_menu.is_none()
                            && app.action_menu.is_none()
                            && app.main_menu.is_none(),
                        "{case}"
                    );
                    assert!(!app.settings && !app.help && app.detail.is_none(), "{case}");
                    assert!(
                        app.prompt.is_none()
                            && app.signal.is_none()
                            && app.pending_plan.is_none()
                            && app.capture.is_none(),
                        "{case}"
                    );
                    assert!(
                        app.signal_targets.is_empty() && app.tag_menu_context.is_none(),
                        "{case}"
                    );
                    assert!(
                        !app.follow_logs
                            && app.log_target.is_none()
                            && !app.detail_search
                            && !app.detail_pending,
                        "{case}"
                    );
                    if ["detail", "searching logs", "help"].contains(&case) {
                        assert_eq!(app.detail_generation, generation + 1);
                    }
                    if case == "settings" {
                        assert_eq!(app.config.theme, "nord");
                    }
                    assert_eq!(
                        app.selected_process, identity,
                        "{case}: background selection changed"
                    );
                    assert_eq!(app.view, View::Processes);
                    assert!(app.filter.is_empty());
                    assert!(input.queued_key.is_none() && input.last_click.is_none());
                });
            }
        }
    }

    #[test]
    fn outside_clicks_close_only_the_current_layer_and_use_resized_bounds() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            app.remember_selection();
            let identity = app.selected_process;
            press(input, app, KeyCode::Char('?'));
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let back = app.main_menu.as_ref().unwrap().back_rect;
            press(input, app, KeyCode::F(2));
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            assert!(!app.popup_rect.contains((back.x, back.y).into()));
            let mut mouse = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: back.x,
                row: back.y,
                modifiers: KeyModifiers::NONE,
            };
            input.mouse(app, mouse);
            assert!(!app.settings && app.main_menu.is_some());
            assert!(input.queued_key.is_none()); // Parent's Back button did not run.
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            mouse.column = 0;
            mouse.row = 0;
            input.mouse(app, mouse);
            assert!(app.main_menu.is_none());
            assert_eq!(app.selected_process, identity);
            press(input, app, KeyCode::F(2));
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let previous = app.popup_rect;
            let mut small = Terminal::new(TestBackend::new(60, 18)).unwrap();
            small.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let resized = app.popup_rect;
            assert!(!previous.contains((resized.x, resized.y).into()));
            mouse.column = resized.x;
            mouse.row = resized.y;
            input.mouse(app, mouse);
            assert!(app.settings); // Inside new geometry, outside old geometry.
            mouse.column = 0;
            mouse.row = 0;
            input.mouse(app, mouse);
            assert!(!app.settings);
        });
    }

    #[test]
    fn main_menu_preserves_selection_and_children_return_without_dispatching_underlying_keys() {
        run(|input, app| {
            app.filter = "worker".into();
            app.move_selection(1);
            let selection = app.selected_process;
            press(input, app, KeyCode::Char('?'));
            for code in [
                KeyCode::Down,
                KeyCode::Right,
                KeyCode::Char('2'),
                KeyCode::Char(' '),
            ] {
                press(input, app, code);
            }
            assert_eq!(app.selected_process, selection);
            assert_eq!(app.filter, "worker");
            assert_eq!(app.view, View::Processes);
            press(input, app, KeyCode::F(1));
            assert!(app.help);
            press(input, app, KeyCode::Esc);
            assert!(!app.help);
            assert!(app.main_menu.is_some());
            press(input, app, KeyCode::F(2));
            assert!(app.settings);
            press(input, app, KeyCode::Right);
            assert_eq!(app.config.theme, "nord");
            press(input, app, KeyCode::Esc);
            assert!(!app.settings);
            assert!(app.main_menu.is_some());
            press(input, app, KeyCode::Char(':'));
            press(input, app, KeyCode::Esc);
            assert!(app.main_menu.is_some());
            press(input, app, KeyCode::Char(':'));
            press(input, app, KeyCode::Char('W'));
            assert!(app.main_menu.is_none());
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::TimeWindow);
            press(input, app, KeyCode::Esc);
            press(input, app, KeyCode::Char('?'));
            press(input, app, KeyCode::Esc);
            assert!(app.main_menu.is_none());
            assert_eq!(app.selected_process, selection);
            assert_eq!(app.filter, "worker");
            press(input, app, KeyCode::Char('?'));
            assert!(!input.key(app, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)));
        });
    }

    #[test]
    fn menu_mouse_hits_include_card_borders_and_background_controls_are_blocked() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        run(|input, app| {
            press(input, app, KeyCode::Char('?'));
            let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let (background, _) = app.footer_actions[0];
            let mut mouse = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: background.x,
                row: background.y,
                modifiers: KeyModifiers::NONE,
            };
            input.mouse(app, mouse);
            assert!(input.queued_key.is_none());
            assert!(app.main_menu.is_none());
            press(input, app, KeyCode::Char('?'));
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let (help, _) = app.main_menu.as_ref().unwrap().hitboxes[1];
            mouse.column = help.right() - 1;
            mouse.row = help.bottom() - 1;
            input.mouse(app, mouse);
            let key = input.queued_key.take().unwrap();
            assert!(input.key(app, key));
            assert!(app.help);
            press(input, app, KeyCode::Esc);
            assert!(app.main_menu.is_some());
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let (source, _) = app.main_menu.as_ref().unwrap().links[1];
            mouse.column = source.right() - 1;
            mouse.row = source.y;
            input.mouse(app, mouse);
            assert_eq!(input.queued_key.take().unwrap().code, KeyCode::Char('g'));
            let back = app.main_menu.as_ref().unwrap().back_rect;
            mouse.column = back.right() - 1;
            mouse.row = back.y;
            input.mouse(app, mouse);
            let key = input.queued_key.take().unwrap();
            assert!(input.key(app, key));
            assert!(app.main_menu.is_none());
        });
    }
    #[test]
    fn view_numbers_remain_stable_during_discovery_and_workload_keys_only_preview_mutations() {
        run(|input, app| {
            for view in app.views() {
                press(input, app, view.key());
                assert_eq!(app.view, view);
            }
            press(input, app, KeyCode::Char('4'));
            assert_eq!(app.view, View::Storage);
            app.snapshot.gpus.push(Gpu::default());
            press(input, app, KeyCode::Char('4'));
            assert_eq!(app.view, View::Storage);
            app.view = View::Services;
            app.snapshot.inventory.services.push(Service {
                name: "rstop-test-not-executed.service".into(),
                scope: "user".into(),
                ..Default::default()
            });
            press(input, app, KeyCode::Char('M'));
            assert_eq!(app.workload_sort, Some(Sort::Memory));
            press(input, app, KeyCode::Char('P'));
            assert_eq!(app.workload_sort, Some(Sort::Cpu));
            press(input, app, KeyCode::Char('m'));
            press(input, app, KeyCode::End);
            assert_eq!(
                app.action_menu.as_ref().unwrap().1,
                app.action_menu.as_ref().unwrap().0.len() - 1
            );
            press(input, app, KeyCode::Home);
            assert_eq!(app.action_menu.as_ref().unwrap().1, 0);
            press(input, app, KeyCode::Char('r'));
            assert!(app.pending_plan.as_ref().unwrap().changes_state);
            assert!(app.prompt.is_some());
            press(input, app, KeyCode::Esc);
            assert!(app.pending_plan.is_none());
        });
    }
    #[test]
    fn layout_changes_never_change_read_only_or_workload_shortcut_meaning() {
        for read_only in [false, true] {
            for layout in ["balanced", "compute", "storage", "minimal"] {
                run(|input, app| {
                    input.read_only = read_only;
                    app.config.layout = layout.into();
                    press(input, app, KeyCode::F(2));
                    assert!(app.settings);
                    app.setting_index = 7;
                    press(input, app, KeyCode::Right);
                    assert_eq!(input.read_only, read_only);
                    press(input, app, KeyCode::Esc);
                    app.set_view(View::Services);
                    app.snapshot.inventory.services.push(Service {
                        name: "parity-test.service".into(),
                        scope: "user".into(),
                        ..Default::default()
                    });
                    press(input, app, KeyCode::Char('m'));
                    let plans = &app.action_menu.as_ref().unwrap().0;
                    assert!(!plans.is_empty());
                    assert!(
                        plans
                            .iter()
                            .all(|(_, plan)| !read_only || !plan.changes_state)
                    );
                    assert!(read_only || plans.iter().any(|(_, plan)| plan.changes_state));
                    assert_eq!(input.read_only, read_only);
                    assert!(!app.settings);
                });
            }
        }
    }
    #[test]
    fn added_group_expansion_and_process_priority_controls_preserve_view_and_mode_boundaries() {
        for read_only in [false, true] {
            run(|input, app| {
                input.read_only = read_only;
                app.snapshot.processes.push(Process {
                    pid: 43,
                    name: "worker".into(),
                    start_time: 2,
                    ..Default::default()
                });
                press(input, app, KeyCode::F(5));
                app.ui_menu.as_mut().unwrap().selected = 2;
                press(input, app, KeyCode::Enter);
                assert!(app.process_grouped());
                assert!(app.selected_process().is_none());
                press(input, app, KeyCode::Enter);
                assert!(!app.process_grouped());
                assert_eq!(app.process_rows().len(), 2);
                assert!(app.selected_process().is_none());
                app.move_selection(1);
                for key in ['I', 'A'] {
                    press(input, app, KeyCode::Char(key));
                    assert_eq!(app.prompt.is_some(), !read_only);
                    if app.prompt.is_some() {
                        press(input, app, KeyCode::Esc);
                    }
                }
                app.set_view(View::Overview);
                press(input, app, KeyCode::F(5));
                app.ui_menu.as_mut().unwrap().selected = 1;
                press(input, app, KeyCode::Enter);
                assert_eq!(app.config.display.overview_panel, "cpu");
                assert_eq!(input.read_only, read_only);
                press(input, app, KeyCode::F(2));
                press(input, app, KeyCode::End);
                assert_eq!(app.setting_index, Config::SETTINGS_COUNT - 1);
                press(input, app, KeyCode::Right);
                assert_eq!(input.read_only, read_only);
                assert_eq!(app.config.layout, "balanced");
            });
        }
    }
}
