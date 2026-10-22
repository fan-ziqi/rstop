pub mod commands;
pub mod dialogs;
pub mod editor;
pub mod menu;
pub mod panes;
mod processes;
pub mod ros;
pub mod signals;
mod sort;
pub mod tag_context;
pub mod text_selection;
use crate::{config::Config, model::*};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Overview,
    Processes,
    Gpu,
    Storage,
    Network,
    System,
    Containers,
    Services,
    Explorer,
    History,
    Events,
}
impl View {
    pub fn key(self) -> crossterm::event::KeyCode {
        crossterm::event::KeyCode::Char(match self {
            Self::Overview => '1',
            Self::Processes => '2',
            Self::Gpu => '3',
            Self::Storage => '4',
            Self::Network => '5',
            Self::System => '6',
            Self::Containers => '7',
            Self::Services => '8',
            Self::Explorer => '6',
            Self::History => '0',
            Self::Events => 'E',
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Processes => "Processes",
            Self::Gpu => "GPU",
            Self::Storage => "Storage",
            Self::Network => "Network",
            Self::System => "System",
            Self::Containers => "Containers",
            Self::Services => "Services",
            Self::Explorer => "Explorer",
            Self::History => "History",
            Self::Events => "Events",
        }
    }
}

pub use sort::Sort;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuKind {
    #[default]
    Actions,
    Columns,
    ProcessSort,
    WorkloadSort,
    ConnectionSort,
    InterfaceSort,
    Display,
    TimeWindow,
    TagActions,
    TagFilter,
    Ros,
}

#[derive(Clone, Debug)]
pub struct UiMenu {
    pub kind: MenuKind,
    pub title: String,
    pub items: Vec<(String, crossterm::event::KeyCode)>,
    pub selected: usize,
    pub rect: ratatui::layout::Rect,
    pub offset: usize,
}
impl UiMenu {
    pub fn new(
        title: &str,
        items: Vec<(String, crossterm::event::KeyCode)>,
        selected: usize,
    ) -> Self {
        Self {
            kind: MenuKind::Actions,
            title: title.into(),
            items,
            selected,
            rect: Default::default(),
            offset: 0,
        }
    }
    pub fn with_reverse(mut self) -> Self {
        self.items.push((
            "Reverse current order".into(),
            crossterm::event::KeyCode::Char('r'),
        ));
        self
    }
    pub fn with_kind(mut self, kind: MenuKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Clone, Debug)]
pub struct SignalTarget {
    pub pid: u32,
    pub start_time: u64,
    pub name: String,
    pub signal: sysinfo::Signal,
}

pub use crate::history::{GpuHistory, History};

pub struct App {
    pub main_menu: Option<menu::MainMenu>,
    pub config: Config,
    pub events: crate::events::Events,
    pub tagged: HashSet<(u32, u64)>,
    pub signal_targets: Vec<SignalTarget>,
    pub ui_menu: Option<UiMenu>,
    pub action_menu_rect: ratatui::layout::Rect,
    pub action_menu_offset: usize,
    pub gpu_section: usize,
    pub network_section: usize,
    pub connection_sort: usize,
    pub interface_sort: usize,
    pub connection_reverse: bool,
    pub interface_reverse: bool,
    pub network_sort_headers: Vec<(ratatui::layout::Rect, usize)>,
    pub network_panes: [panes::PaneState; 2],
    pub gpu_panes: [panes::PaneState; 3],
    pub panel_regions: Vec<panes::PaneRegion>,
    pub panel_focused: bool,
    pub selected_interface: Option<String>,
    pub selected_connection: Option<String>,
    pub capture: Option<crate::network_tools::Capture>,
    pub workload_sort: Option<Sort>,
    pub gpu_ranges: Vec<(ratatui::layout::Rect, usize)>,
    pub gpu_card_offset: usize,
    pub filter_rect: ratatui::layout::Rect,
    pub footer_actions: Vec<(ratatui::layout::Rect, crossterm::event::KeyEvent)>,
    pub action_menu: Option<(Vec<(String, crate::management::Plan)>, usize)>,
    pub pending_plan: Option<crate::management::Plan>,
    pub prompt: Option<(String, String, Option<Process>)>,
    pub update_notice: Option<crate::updates::Notice>,
    pub update_buttons: Vec<(ratatui::layout::Rect, usize)>,
    pub replay: bool,
    pub column_offset: usize,
    pub visible_columns: Vec<String>,
    pub horizontal_scroll: usize,
    pub snapshot: Snapshot,
    pub applications: Vec<crate::applications::Application>,
    pub tag_menu_context: Option<tag_context::TagContext>,
    pub ros_browser: Option<ros::Browser>,
    pub object_scope: Option<tag_context::ObjectScope>,
    pub label_hits: Vec<ratatui::layout::Rect>,
    pub label_targets: Vec<(ratatui::layout::Rect, String)>,
    pub clicked_label: Option<String>,
    pub history: History,
    pub view: View,
    pub selected: usize,
    pub selected_process: Option<(u32, u64)>,
    pub process_order: std::collections::HashMap<(u32, u64), usize>,
    pub process_history: crate::history::ProcessHistory,
    pub process_detail_rect: ratatui::layout::Rect,
    pub selected_workload: Option<(View, String, String)>,
    pub selection_open: bool,
    pub selection_scroll: usize,
    pub selection_lines: usize,
    pub selection_viewport: usize,
    pub text_regions: Vec<text_selection::TextRegion>,
    pub text_selection: text_selection::TextSelection,
    pub clipboard_request: Option<String>,
    pub selected_gpu: usize,
    pub filter: String,
    pub editing_filter: bool,
    pub editor: editor::Editor,
    pub filter_before_edit: String,
    pub controls_rect: ratatui::layout::Rect,
    pub controls: Vec<(ratatui::layout::Rect, crossterm::event::KeyCode)>,
    pub sort: Sort,
    pub reverse: bool,
    pub tree: bool,
    pub collapsed: HashSet<u32>,
    pub gpu_only: bool,
    pub own_only: bool,
    pub help: bool,
    pub settings: bool,
    pub setting_index: usize,
    pub setting_offset: usize,
    pub sort_headers: Vec<(ratatui::layout::Rect, Sort)>,
    pub settings_rect: ratatui::layout::Rect,
    /// Outer bounds of the topmost popup, registered by its renderer each frame.
    pub popup_rect: ratatui::layout::Rect,
    pub popup_actions: Vec<(ratatui::layout::Rect, crossterm::event::KeyCode)>,
    pub dialog: dialogs::State,
    pub signal_ui: signals::State,
    pub action_results: std::sync::Arc<std::sync::Mutex<VecDeque<String>>>,
    pub detail: Option<(String, String)>,
    pub detail_scroll: usize,
    pub detail_rows: usize,
    pub detail_width: u16,
    pub detail_column: u16,
    pub detail_filter: String,
    pub detail_search: bool,
    pub log_target: Option<crate::logs::Target>,
    pub detail_generation: u64,
    pub detail_pending: bool,
    pub follow_logs: bool,
    pub signal: Option<SignalTarget>,
    pub status: String,
    pub graph_window: usize,
    pub table_offset: usize,
    pub table_rect: ratatui::layout::Rect,
    pub tab_ranges: Vec<(ratatui::layout::Rect, View)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let graph_window = config.history;
        let sort = Sort::from_stored(config.sort_index);
        let reverse = config.reverse_sort;
        let tree = config.tree;
        Self {
            main_menu: None,
            config,
            events: Default::default(),
            tagged: Default::default(),
            signal_targets: vec![],
            ui_menu: None,
            action_menu_rect: Default::default(),
            action_menu_offset: 0,
            gpu_section: 0,
            network_section: 0,
            connection_sort: 0,
            interface_sort: 0,
            connection_reverse: false,
            interface_reverse: false,
            network_sort_headers: vec![],
            network_panes: Default::default(),
            gpu_panes: Default::default(),
            panel_regions: vec![],
            panel_focused: true,
            selected_interface: None,
            selected_connection: None,
            capture: None,
            workload_sort: None,
            gpu_ranges: vec![],
            gpu_card_offset: 0,
            filter_rect: Default::default(),
            footer_actions: vec![],
            action_menu: None,
            pending_plan: None,
            prompt: None,
            update_notice: None,
            update_buttons: vec![],
            replay: false,
            column_offset: 0,
            visible_columns: vec![],
            horizontal_scroll: 0,
            snapshot: Snapshot::default(),
            applications: vec![],
            tag_menu_context: None,
            ros_browser: None,
            object_scope: None,
            label_hits: vec![],
            label_targets: vec![],
            clicked_label: None,
            history: History::default(),
            view: View::Overview,
            selected: 0,
            selected_process: None,
            process_order: Default::default(),
            process_history: Default::default(),
            process_detail_rect: Default::default(),
            selected_workload: None,
            selection_open: false,
            selection_scroll: 0,
            selection_lines: 0,
            selection_viewport: 0,
            text_regions: vec![],
            text_selection: Default::default(),
            clipboard_request: None,
            selected_gpu: 0,
            filter: String::new(),
            editing_filter: false,
            editor: Default::default(),
            filter_before_edit: String::new(),
            controls_rect: Default::default(),
            controls: vec![],
            sort,
            reverse,
            tree,
            collapsed: HashSet::new(),
            gpu_only: false,
            own_only: false,
            help: false,
            settings: false,
            setting_index: 0,
            setting_offset: 0,
            sort_headers: vec![],
            settings_rect: Default::default(),
            popup_rect: Default::default(),
            action_results: Default::default(),
            detail: None,
            detail_scroll: 0,
            detail_rows: 20,
            detail_width: 108,
            detail_column: 0,
            detail_filter: String::new(),
            detail_search: false,
            log_target: None,
            detail_generation: 0,
            detail_pending: false,
            follow_logs: false,
            signal: None,
            popup_actions: vec![],
            dialog: Default::default(),
            signal_ui: Default::default(),
            status: "Discovering host…".into(),
            graph_window,
            table_offset: 0,
            table_rect: ratatui::layout::Rect::default(),
            tab_ranges: vec![],
        }
    }

    pub fn views(&self) -> Vec<View> {
        vec![
            View::Overview,
            View::Processes,
            View::Gpu,
            View::Storage,
            View::Network,
            View::System,
            View::Containers,
            View::Services,
        ]
    }

    pub fn ingest(&mut self, snapshot: Snapshot) {
        let mut snapshot = snapshot;
        let applications = crate::tags::annotate(&mut snapshot);
        self.ingest_annotated(snapshot, applications);
    }

    /// Accept one sample already annotated by the sampler (including recording/replay).
    pub fn ingest_annotated(
        &mut self,
        mut snapshot: Snapshot,
        applications: Vec<crate::applications::Application>,
    ) {
        snapshot
            .alerts
            .extend(crate::diagnostics::resource_alerts(&snapshot, &self.config));
        self.history.ingest(&snapshot, &self.config);
        let changes = self.events.update(
            snapshot.timestamp,
            &snapshot.alerts,
            self.config.alert_hold_seconds,
            self.config.alert_history,
        );
        if !self.replay && self.config.config_exec && !self.config.alert_commands.is_empty() {
            for event in changes.into_iter().filter(|e| e.active).take(8) {
                for args in self.config.alert_commands.clone().into_iter().take(8) {
                    let condition = event.condition.clone();
                    let log = self.action_results.clone();
                    std::thread::spawn(move || {
                        if let Some(program) = args.first() {
                            let argv: Vec<_> = args
                                .iter()
                                .skip(1)
                                .map(|arg| arg.replace("{condition}", &condition))
                                .collect();
                            let refs: Vec<_> = argv.iter().map(String::as_str).collect();
                            let result = crate::discovery::command(
                                program,
                                &refs,
                                std::time::Duration::from_secs(3),
                            );
                            if let Ok(mut log) = log.lock() {
                                log.push_back(format!(
                                    "{} · {} · {}",
                                    crate::collector::unix_time(),
                                    condition,
                                    result
                                        .map(|output| format!("{program}: {output}"))
                                        .unwrap_or_else(|e| format!("{program}: {e}"))
                                ));
                                while log.len() > 100 {
                                    log.pop_front();
                                }
                            }
                        }
                        let _ = condition;
                    });
                }
            }
        }
        if let Ok(log) = self.action_results.lock()
            && !log.is_empty()
        {
            snapshot
                .extra
                .insert("Alert command results".into(), serde_json::json!(*log));
        }
        self.tagged.retain(|identity| {
            snapshot
                .processes
                .iter()
                .any(|p| (p.pid, p.start_time) == *identity)
        });
        self.applications = applications;
        self.snapshot = snapshot;
        self.selected_gpu = self
            .selected_gpu
            .min(self.snapshot.gpus.len().saturating_sub(1));
        if !self.views().contains(&self.view) {
            self.view = View::Overview;
            self.selected = 0;
            self.clear_process_selection();
        }
        let gpu_focus = self.gpu_section;
        if self.view == View::Gpu {
            self.set_gpu_section(0);
        }
        self.reconcile_selection();
        if self.view == View::Gpu {
            self.set_gpu_section(gpu_focus);
        }
        if self.view == View::Network
            && self.network_section == 0
            && let Some(id) = &self.selected_connection
        {
            if let Some(index) = self.filtered_connections().iter().position(|c| &c.id == id) {
                self.selected = index;
            } else {
                self.selected_connection = None;
                self.close_selection();
                self.selected = 0;
            }
        }
        if self.view == View::Network
            && self.network_section == 1
            && let Some(name) = &self.selected_interface
        {
            self.selected = self
                .filtered_networks()
                .iter()
                .position(|n| &n.name == name)
                .unwrap_or(0);
        }
        if let Some((view, scope, name)) = &self.selected_workload {
            let index = match view {
                View::Services if self.view == *view => self
                    .filtered_services()
                    .iter()
                    .position(|s| &s.scope == scope && &s.name == name),
                View::Containers if self.view == *view => self
                    .filtered_containers()
                    .iter()
                    .position(|c| &c.runtime == scope && &c.id == name),
                _ => None,
            };
            if let Some(index) = index {
                self.selected = index;
            } else {
                self.selected_workload = None;
                self.close_selection();
            }
        }
        if let Some(process) = self.selected_process().cloned() {
            let gpu = self.gpu_metrics(&process);
            self.process_history
                .push(&process, gpu, self.config.history);
        }
    }

    pub fn set_view(&mut self, view: View) {
        self.object_scope = None;
        self.view = match view {
            View::Explorer => View::System,
            View::History | View::Events => View::Overview,
            view => view,
        };
        self.selected = 0;
        self.clear_row_selection();
        self.network_panes = Default::default();
        self.gpu_panes = Default::default();
        self.panel_regions.clear();
        self.table_offset = 0;
        self.filter.clear();
        self.editing_filter = false;
    }
    pub fn can_filter(&self) -> bool {
        self.has_process_table()
            || matches!(
                self.view,
                View::Containers
                    | View::Services
                    | View::Explorer
                    | View::History
                    | View::Events
                    | View::Network
            )
            || (self.view == View::Gpu && self.gpu_section == 2)
    }
    pub fn begin_filter(&mut self) {
        if !self.can_filter() {
            return;
        }
        self.filter_before_edit = self.filter.clone();
        self.clear_active_selection();
        self.editor.reset();
        self.editing_filter = true;
    }
    pub fn plot_window(&self) -> crate::plot::Window<'_> {
        if self.config.time_window_seconds > 0 {
            self.history.time_window(self.config.time_window_seconds)
        } else {
            self.history.window(self.graph_window)
        }
    }
    pub fn set_time_window(&mut self, seconds: u64) {
        let seconds = seconds.min(3600 * self.config.interval_ms / 1000);
        self.config.time_window_seconds = seconds;
        let needed = (seconds * 1000).div_ceil(self.config.interval_ms) as usize;
        self.config.history = self.config.history.max(needed.min(3600));
        self.graph_window = self.config.history;
    }
    pub fn time_windows(&self) -> Vec<u64> {
        let max = 3600 * self.config.interval_ms / 1000;
        let mut values: Vec<_> = [15, 30, 60, 180, 300, 900, 1800, 3600]
            .into_iter()
            .filter(|s| *s <= max)
            .collect();
        let current = self.config.time_window_seconds;
        if current > 0 && current <= max && !values.contains(&current) {
            values.push(current);
            values.sort_unstable();
        }
        values.push(0);
        values
    }

    pub fn extra_rows(&self) -> Vec<(String, serde_json::Value)> {
        let filter = self.filter.to_lowercase();
        self.snapshot
            .extra
            .iter()
            .chain(self.snapshot.inventory.extra.iter())
            .filter(|(key, _)| key.to_lowercase().contains(&filter))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    pub fn filtered_connections(&self) -> Vec<&crate::connections::Connection> {
        let mut rows: Vec<_> = self
            .snapshot
            .connections
            .connections
            .iter()
            .filter(|c| {
                self.object_scope
                    .as_ref()
                    .is_none_or(|scope| scope.connections.contains(&c.id))
                    && crate::network_tools::matches(c, &self.filter)
            })
            .collect();
        rows.sort_by(|a, b| {
            crate::network_sort::ConnectionSort::at(self.connection_sort).compare(
                a,
                b,
                self.connection_reverse,
            )
        });
        rows
    }
    pub fn filtered_networks(&self) -> Vec<&Network> {
        let filter = self.filter.to_lowercase();
        let mut rows: Vec<_> = self
            .snapshot
            .networks
            .iter()
            .filter(|n| self.config.show_loopback || !is_loopback(&n.name))
            .filter(|n| {
                format!("{} {} {}", n.name, n.address, n.state)
                    .to_lowercase()
                    .contains(&filter)
            })
            .collect();
        rows.sort_by(|a, b| {
            crate::network_sort::InterfaceSort::at(self.interface_sort).compare(
                a,
                b,
                self.interface_reverse,
            )
        });
        rows
    }
    pub fn connection_sort_label(&self) -> &'static str {
        crate::network_sort::ConnectionSort::at(self.connection_sort).header()
    }
    pub fn network_sort_label(&self) -> &'static str {
        if self.network_section == 0 {
            self.connection_sort_label()
        } else {
            crate::network_sort::InterfaceSort::at(self.interface_sort).header()
        }
    }
    pub fn network_sort_descending(&self) -> bool {
        if self.network_section == 0 {
            crate::network_sort::ConnectionSort::at(self.connection_sort)
                .descending(self.connection_reverse)
        } else {
            crate::network_sort::InterfaceSort::at(self.interface_sort)
                .descending(self.interface_reverse)
        }
    }
    pub fn set_network_sort(&mut self, index: usize, toggle: bool) {
        if self.network_section == 0 {
            if toggle && self.connection_sort == index {
                self.connection_reverse = !self.connection_reverse;
            } else {
                self.connection_sort = index;
                self.connection_reverse = false;
            }
            self.selected = self
                .selected_connection
                .as_ref()
                .and_then(|id| self.filtered_connections().iter().position(|c| &c.id == id))
                .unwrap_or(0);
        } else {
            if toggle && self.interface_sort == index {
                self.interface_reverse = !self.interface_reverse;
            } else {
                self.interface_sort = index;
                self.interface_reverse = false;
            }
            self.selected = self
                .selected_interface
                .as_ref()
                .and_then(|id| self.filtered_networks().iter().position(|n| &n.name == id))
                .unwrap_or(0);
        }
        self.table_offset = 0;
    }
    pub fn has_process_table(&self) -> bool {
        self.view == View::Processes
            || (self.view == View::Overview
                && self.config.display.overview_panel == "all"
                && self
                    .config
                    .custom_layout
                    .as_ref()
                    .is_some_and(|layout| layout.contains_panel("processes")))
            || (self.view == View::Gpu && self.gpu_section == 0)
    }
    pub fn filtered_containers(&self) -> Vec<&Container> {
        let filter = self.filter.to_lowercase();
        let mut rows: Vec<_> = self
            .snapshot
            .inventory
            .containers
            .iter()
            .filter(|c| {
                crate::tags::query_matches(
                    &c.tags,
                    &filter,
                    &format!("{} {} {} {} {}", c.name, c.image, c.state, c.runtime, c.id),
                )
            })
            .collect();
        if let Some(sort) = self.workload_sort {
            rows.sort_by(|a, b| {
                let order = match sort {
                    Sort::Cpu => b
                        .cpu
                        .partial_cmp(&a.cpu)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    Sort::Memory => b.memory.cmp(&a.memory),
                    Sort::State => a.state.cmp(&b.state),
                    _ => a.name.cmp(&b.name),
                }
                .then(a.name.cmp(&b.name));
                if self.reverse { order.reverse() } else { order }
            });
        }
        rows
    }

    pub fn filtered_services(&self) -> Vec<&Service> {
        let filter = self.filter.to_lowercase();
        let mut rows: Vec<_> = self
            .snapshot
            .inventory
            .services
            .iter()
            .filter(|s| {
                crate::tags::query_matches(
                    &s.tags,
                    &filter,
                    &format!("{} {} {} {}", s.name, s.scope, s.active, s.description),
                )
            })
            .collect();
        if let Some(sort) = self.workload_sort {
            rows.sort_by(|a, b| {
                let order = match sort {
                    Sort::Cpu => b
                        .cpu_percent
                        .partial_cmp(&a.cpu_percent)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    Sort::Memory => b.memory.cmp(&a.memory),
                    Sort::State => a.active.cmp(&b.active),
                    _ => a.name.cmp(&b.name),
                }
                .then(a.name.cmp(&b.name));
                if self.reverse { order.reverse() } else { order }
            });
        }
        rows
    }

    pub fn row_count(&self) -> usize {
        if self.process_grouped() {
            return self.program_groups().len();
        }
        match self.view {
            View::Gpu if self.gpu_section == 2 => self
                .snapshot
                .gpus
                .get(self.selected_gpu)
                .map_or(0, |gpu| crate::fields::gpu(gpu, &self.filter).len()),
            View::Gpu if self.gpu_section == 1 => 0,
            View::Overview if !self.has_process_table() => 0,
            View::Overview | View::Processes | View::Gpu => self.process_rows().len(),
            View::Containers => self.filtered_containers().len(),
            View::Services => self.filtered_services().len(),
            View::Explorer => self.extra_rows().len(),
            View::History => self
                .history
                .series
                .keys()
                .filter(|key| key.to_lowercase().contains(&self.filter.to_lowercase()))
                .count(),
            View::Events => self
                .events
                .history
                .iter()
                .filter(|event| crate::filter::event_matches(event, &self.filter))
                .count(),
            View::Storage => self.snapshot.disks.len().max(self.snapshot.disk_io.len()),
            View::Network if self.network_section == 0 => self.filtered_connections().len(),
            View::Network => self.filtered_networks().len(),
            View::System => self
                .snapshot
                .sensors
                .len()
                .max(self.snapshot.inventory.listeners.len())
                .max(self.snapshot.inventory.notes.len()),
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.has_process_table() && !self.process_grouped() && self.selected_process.is_none() {
            self.selected = self.table_offset.min(self.row_count().saturating_sub(1));
            self.remember_selection();
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.row_count().saturating_sub(1));
        self.remember_selection();
    }

    pub fn remember_selection(&mut self) {
        if self.process_grouped() {
            self.clear_process_selection();
            return;
        }
        if self.view == View::Network && self.network_section == 1 {
            self.selected_interface = self
                .filtered_networks()
                .get(self.selected)
                .map(|n| n.name.clone());
        }
        if self.view == View::Network && self.network_section == 0 {
            let identity = self
                .filtered_connections()
                .get(self.selected)
                .map(|c| c.id.clone());
            if identity != self.selected_connection {
                self.close_selection();
            }
            self.selected_connection = identity;
        }
        if !self.has_process_table() {
            let workload = match self.view {
                View::Services => self
                    .filtered_services()
                    .get(self.selected)
                    .map(|s| (self.view, s.scope.clone(), s.name.clone())),
                View::Containers => self
                    .filtered_containers()
                    .get(self.selected)
                    .map(|c| (self.view, c.runtime.clone(), c.id.clone())),
                _ => None,
            };
            if workload != self.selected_workload {
                self.close_selection();
            }
            self.selected_workload = workload;
            if self.view != View::Gpu {
                self.clear_process_selection();
            }
            return;
        }
        let identity = self.process_rows().get(self.selected).map(|(i, _)| {
            let p = &self.snapshot.processes[*i];
            (p.pid, p.start_time)
        });
        if identity != self.selected_process {
            self.close_selection();
            self.process_history = Default::default();
        }
        self.selected_process = identity;
        let height = self.table_rect.height.max(1) as usize;
        if self.selected < self.table_offset {
            self.table_offset = self.selected;
        } else if self.selected >= self.table_offset + height {
            self.table_offset = self.selected + 1 - height;
        }
        if let Some(identity) = identity
            && !self.process_order.contains_key(&identity)
        {
            self.process_order = self
                .process_rows()
                .iter()
                .enumerate()
                .map(|(rank, (i, _))| {
                    let p = &self.snapshot.processes[*i];
                    ((p.pid, p.start_time), rank)
                })
                .collect();
        }
        if self.process_history.cpu.is_empty()
            && let Some(p) = self.selected_process().cloned()
        {
            self.process_history
                .push(&p, self.gpu_metrics(&p), self.config.history);
        }
    }

    pub fn clear_row_selection(&mut self) {
        self.clear_process_selection();
        self.selected_workload = None;
        self.selected_connection = None;
        self.close_selection();
    }
    pub fn close_selection(&mut self) {
        self.text_selection.clear();
        self.selection_open = false;
        self.process_detail_rect = Default::default();
        self.selection_scroll = 0;
        self.selection_lines = 0;
        self.selection_viewport = 0;
    }
    /// Open sampled details only after explicit activation of a table row.
    pub fn open_selection(&mut self) -> bool {
        if self.has_process_table() && !self.process_grouped() {
            self.selection_open = self.selected_process().is_some();
        } else if matches!(self.view, View::Services | View::Containers)
            || self.view == View::Network && self.network_section == 0
        {
            self.remember_selection();
            self.selection_open = self.has_row_selection();
        } else {
            return false;
        }
        self.selection_open
    }
    pub fn has_row_selection(&self) -> bool {
        self.has_process_table() && self.selected_process.is_some()
            || self.selected_workload.is_some()
            || self.view == View::Network
                && self.network_section == 0
                && self.selected_connection.is_some()
    }
    pub fn scroll_selection(&mut self, delta: isize) {
        if matches!(self.view, View::Network | View::Gpu) && self.focused_panel() != 0 {
            let focus = self.focused_panel();
            self.focus_panel(0);
            self.scroll_selection(delta);
            self.focus_panel(focus);
            return;
        }

        self.selection_scroll = self
            .selection_scroll
            .saturating_add_signed(delta)
            .min(self.selection_lines.saturating_sub(self.selection_viewport));
    }

    pub fn clear_process_selection(&mut self) {
        if self.selected_process.is_some() {
            self.close_selection();
        }
        self.selected_process = None;
        self.process_order.clear();
        self.process_history = Default::default();
        self.process_detail_rect = Default::default();
    }

    pub fn resort_processes(&mut self) {
        self.process_order.clear();
        self.reconcile_selection();
        if self.selected_process.is_some() {
            self.remember_selection();
        }
    }

    fn reconcile_selection(&mut self) {
        if self.process_grouped() {
            self.clear_process_selection();
            self.selected = self.selected.min(self.row_count().saturating_sub(1));
            return;
        }
        if !self.has_process_table() {
            self.clear_process_selection();
            self.selected = self.selected.min(self.row_count().saturating_sub(1));
            return;
        }
        if self.selected_process.is_none() {
            return;
        }
        let rows = self.process_rows();
        if self.has_process_table()
            && let Some(identity) = self.selected_process
        {
            if let Some(index) = rows.iter().position(|(i, _)| {
                let p = &self.snapshot.processes[*i];
                (p.pid, p.start_time) == identity
            }) {
                self.selected = index;
            } else {
                self.clear_process_selection();
            }
        }
        self.selected = self.selected.min(self.row_count().saturating_sub(1));
    }
}

pub use crate::history::gpu_key;
pub use crate::model::is_loopback;
