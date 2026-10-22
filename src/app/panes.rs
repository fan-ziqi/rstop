//! Independent table state for panels that share one resource page.
use super::{App, Sort, View};
use ratatui::layout::Rect;

#[derive(Clone, Default)]
pub struct PaneState {
    selected: usize,
    offset: usize,
    filter: String,
    filter_before_edit: String,
    cursor: Option<usize>,
    selection_scroll: usize,
}
impl PaneState {
    pub fn capture(app: &App) -> Self {
        Self {
            selected: app.selected,
            offset: app.table_offset,
            filter: app.filter.clone(),
            filter_before_edit: app.filter_before_edit.clone(),
            cursor: app.editor.cursor,
            selection_scroll: app.selection_scroll,
        }
    }
    fn apply(self, app: &mut App) {
        app.selected = self.selected;
        app.table_offset = self.offset;
        app.filter = self.filter;
        app.filter_before_edit = self.filter_before_edit;
        app.editor.cursor = self.cursor;
        app.selection_scroll = self.selection_scroll;
        app.editing_filter = false;
    }
}

#[derive(Clone)]
pub struct PaneRegion {
    pub section: usize,
    pub area: Rect,
    pub table: Rect,
    pub filter: Rect,
    pub controls: Rect,
    pub network_headers: Vec<(Rect, usize)>,
    pub process_headers: Vec<(Rect, Sort)>,
}
impl PaneRegion {
    pub fn capture(app: &App, area: Rect, section: usize) -> Self {
        Self {
            section,
            area,
            table: app.table_rect,
            filter: app.filter_rect,
            controls: app.controls_rect,
            network_headers: app.network_sort_headers.clone(),
            process_headers: app.sort_headers.clone(),
        }
    }
    pub fn apply(&self, app: &mut App) {
        app.table_rect = self.table;
        app.filter_rect = self.filter;
        app.controls_rect = self.controls;
        app.network_sort_headers.clone_from(&self.network_headers);
        app.sort_headers.clone_from(&self.process_headers);
    }
}

impl App {
    pub fn focused_panel(&self) -> usize {
        if self.view == View::Network {
            self.network_section
        } else {
            self.gpu_section
        }
    }
    pub fn focus_panel(&mut self, section: usize) {
        match self.view {
            View::Network => self.set_network_section(section),
            View::Gpu => self.set_gpu_section(section),
            _ => return,
        }
        if let Some(region) = self
            .panel_regions
            .iter()
            .find(|r| r.section == self.focused_panel())
            .cloned()
        {
            region.apply(self);
        }
    }
    /// A click or wheel over a visible panel targets that panel before dispatch.
    pub fn focus_panel_at(&mut self, x: u16, y: u16) {
        if let Some(section) = self
            .panel_regions
            .iter()
            .find(|r| r.area.contains((x, y).into()))
            .map(|r| r.section)
        {
            self.focus_panel(section);
        }
    }
    pub fn set_network_section(&mut self, section: usize) {
        let section = section % 2;
        if section == self.network_section {
            return;
        }
        self.network_panes[self.network_section] = PaneState::capture(self);
        self.network_section = section;
        self.network_panes[section].clone().apply(self);
        if section == 0 {
            if let Some(id) = &self.selected_connection {
                if let Some(index) = self.filtered_connections().iter().position(|c| &c.id == id) {
                    self.selected = index;
                } else {
                    self.selected_connection = None;
                }
            }
        } else if let Some(name) = &self.selected_interface {
            if let Some(index) = self
                .filtered_networks()
                .iter()
                .position(|n| &n.name == name)
            {
                self.selected = index;
            } else {
                self.selected_interface = None;
            }
        }
        self.selected = self.selected.min(self.row_count().saturating_sub(1));
    }
    pub fn set_gpu_section(&mut self, section: usize) {
        // Legacy History selects the process panel; charts are always on the page.
        let section = if section == 2 { 2 } else { 0 };
        if section == self.gpu_section {
            return;
        }
        self.gpu_panes[self.gpu_section.min(2)] = PaneState::capture(self);
        self.gpu_section = section;
        self.gpu_panes[section].clone().apply(self);
        self.selected = self.selected.min(self.row_count().saturating_sub(1));
    }
    pub fn gpu_info_open(&self) -> bool {
        self.view == View::Gpu && self.gpu_section == 2
    }
    pub fn jump_selection(&mut self, end: bool) {
        let focus = self.focused_panel();
        self.focus_panel(0);
        self.selection_scroll = if end {
            self.selection_lines.saturating_sub(self.selection_viewport)
        } else {
            0
        };
        self.focus_panel(focus);
    }
    pub fn clear_active_selection(&mut self) {
        match self.view {
            View::Network if self.network_section == 1 => self.selected_interface = None,
            View::Gpu if self.gpu_section == 2 => {}
            _ => self.clear_row_selection(),
        }
    }
}
