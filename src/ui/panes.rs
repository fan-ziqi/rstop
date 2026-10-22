//! Render every panel, restoring the active panel's input geometry afterwards.
use super::{Palette, controls};
use crate::app::{App, panes::PaneRegion};
use ratatui::prelude::*;

pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    section: usize,
    app: &mut App,
    p: Palette,
    render: impl FnOnce(&mut Frame, Rect, &mut App, Palette),
) {
    let focus = app.focused_panel();
    let editing = app.editing_filter;
    app.focus_panel(section);
    app.panel_focused = focus == section;
    app.editing_filter = editing && app.panel_focused;
    app.table_rect = Rect::default();
    app.filter_rect = Rect::default();
    app.controls_rect = Rect::default();
    app.network_sort_headers.clear();
    app.sort_headers.clear();
    render(frame, area, app, p);
    controls::table_header(frame, app, p);
    let region = PaneRegion::capture(app, area, section);
    app.panel_regions.push(region);
    app.focus_panel(focus);
    app.panel_focused = true;
    app.editing_filter = editing;
}
