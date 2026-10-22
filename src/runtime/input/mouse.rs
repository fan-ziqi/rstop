use super::Input;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstop::app::App;
use std::time::{Duration, Instant};
impl Input<'_> {
    pub fn mouse(&mut self, app: &mut App, mouse: MouseEvent) {
        self.last_escape = None;
        if app.update_notice.is_some() {
            if matches!(
                mouse.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            ) {
                app.dialog.scroll = if mouse.kind == MouseEventKind::ScrollUp {
                    app.dialog.scroll.saturating_sub(1)
                } else {
                    (app.dialog.scroll + 1).min(app.dialog.lines.saturating_sub(app.dialog.rows))
                };
            }
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Some((_, index)) = app
                    .update_buttons
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                {
                    let key = [
                        KeyCode::Char('u'),
                        KeyCode::Char('i'),
                        KeyCode::Char('n'),
                        KeyCode::Char('r'),
                    ][*index];
                    self.queued_key = Some(KeyEvent::new(key, KeyModifiers::NONE));
                } else if !app.popup_rect.contains((mouse.column, mouse.row).into()) {
                    self.update_key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                }
            }
            return;
        }
        let has_popup = app.ui_menu.is_some()
            || app.action_menu.is_some()
            || app.prompt.is_some()
            || app.signal.is_some()
            || app.settings
            || app.detail.is_some()
            || app.help
            || app.main_menu.is_some()
            || app.gpu_info_open();
        if has_popup
            && !app.popup_rect.is_empty()
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && !app.popup_rect.contains((mouse.column, mouse.row).into())
        {
            // Close the current layer through the same cancellation path as Esc.
            // Searching is part of a detail window, not a separate popup layer.
            if app.ui_menu.is_none()
                && app.action_menu.is_none()
                && app.prompt.is_none()
                && app.signal.is_none()
                && !app.settings
            {
                app.detail_search = false;
                if app.gpu_info_open() {
                    app.editing_filter = false;
                }
            }
            if app
                .ui_menu
                .as_ref()
                .is_some_and(|m| m.kind == rstop::app::MenuKind::Ros)
                && let Some(browser) = &mut app.ros_browser
            {
                browser.searching = false;
            }
            self.last_click = None;
            self.queued_key = None;
            self.key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            return; // Never send this click to the parent popup or resource table.
        }
        if !has_popup
            && matches!(
                mouse.kind,
                MouseEventKind::Down(MouseButton::Left | MouseButton::Right)
                    | MouseEventKind::ScrollUp
                    | MouseEventKind::ScrollDown
            )
            && !app
                .process_detail_rect
                .contains((mouse.column, mouse.row).into())
        {
            app.focus_panel_at(mouse.column, mouse.row);
        }
        if has_popup
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && app.dialog.input.contains((mouse.column, mouse.row).into())
        {
            let value = app
                .prompt
                .as_ref()
                .map(|(_, value, _)| value.clone())
                .unwrap_or_else(|| app.detail_filter.clone());
            let cursor = ratatui::text::Line::raw(&value[..app.editor.position(&value)]).width();
            let scroll = cursor.saturating_sub(app.dialog.input.width.saturating_sub(1) as usize);
            let column = (mouse.column - app.dialog.input.x) as usize + scroll;
            let mut width = 0;
            let mut position = value.len();
            for (index, ch) in value.char_indices() {
                let next = width + ratatui::text::Line::raw(ch.to_string()).width();
                if next > column {
                    position = index;
                    break;
                }
                width = next;
            }
            app.editor.cursor = Some(position);
            if app.prompt.is_none() {
                app.detail_search = true;
            }
            self.last_click = None;
            return;
        }
        if has_popup
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && let Some((_, key)) = app
                .popup_actions
                .iter()
                .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
        {
            if app.gpu_info_open() && app.detail.is_none() && *key == KeyCode::Esc {
                app.editing_filter = false;
            }
            self.queued_key = Some(KeyEvent::new(*key, KeyModifiers::NONE));
            self.last_click = None;
            return;
        }
        if let Some(menu) = &mut app.ui_menu {
            match mouse.kind {
                MouseEventKind::ScrollDown => {
                    menu.selected = (menu.selected + 1).min(menu.items.len().saturating_sub(1))
                }
                MouseEventKind::ScrollUp => menu.selected = menu.selected.saturating_sub(1),
                MouseEventKind::Down(MouseButton::Left)
                    if menu.rect.contains((mouse.column, mouse.row).into()) =>
                {
                    let index = menu.offset + (mouse.row - menu.rect.y) as usize;
                    if index < menu.items.len() {
                        menu.selected = index;
                        self.queued_key = Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
                _ => {}
            }

            return;
        }
        if let Some((items, selected)) = &mut app.action_menu {
            match mouse.kind {
                MouseEventKind::ScrollDown => {
                    *selected = (*selected + 1).min(items.len().saturating_sub(1))
                }
                MouseEventKind::ScrollUp => *selected = selected.saturating_sub(1),
                MouseEventKind::Down(MouseButton::Left)
                    if app
                        .action_menu_rect
                        .contains((mouse.column, mouse.row).into()) =>
                {
                    let index =
                        app.action_menu_offset + (mouse.row - app.action_menu_rect.y) as usize;
                    if index < items.len() {
                        *selected = index;
                        self.queued_key = Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
                _ => {}
            }

            return;
        }
        if app.prompt.is_some() || app.signal.is_some() {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Some((_, signal)) = app
                    .signal_ui
                    .hits
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                    && let Some(target) = &mut app.signal
                {
                    target.signal = *signal;
                    return;
                }
                if let Some((_, code)) = app
                    .popup_actions
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                {
                    self.queued_key = Some(KeyEvent::new(*code, KeyModifiers::NONE));
                }
            } else if app.prompt.is_some()
                && !app.dialog.content.is_empty()
                && matches!(
                    mouse.kind,
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                )
            {
                self.queued_key = Some(KeyEvent::new(
                    if mouse.kind == MouseEventKind::ScrollDown {
                        KeyCode::Down
                    } else {
                        KeyCode::Up
                    },
                    KeyModifiers::NONE,
                ));
            } else if app.signal.is_some()
                && matches!(
                    mouse.kind,
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                )
            {
                if app
                    .signal_ui
                    .targets_rect
                    .contains((mouse.column, mouse.row).into())
                {
                    app.signal_ui.target_scroll = app
                        .signal_ui
                        .target_scroll
                        .saturating_add_signed(if mouse.kind == MouseEventKind::ScrollDown {
                            1
                        } else {
                            -1
                        });
                } else {
                    self.queued_key = Some(KeyEvent::new(
                        if mouse.kind == MouseEventKind::ScrollDown {
                            KeyCode::Down
                        } else {
                            KeyCode::Up
                        },
                        KeyModifiers::NONE,
                    ));
                }
            }
            return;
        }
        if app.gpu_info_open()
            && app.detail.is_none()
            && !app.help
            && !app.settings
            && app.main_menu.is_none()
        {
            match mouse.kind {
                MouseEventKind::ScrollDown => app.move_selection(3),
                MouseEventKind::ScrollUp => app.move_selection(-3),
                MouseEventKind::Down(MouseButton::Left) => {
                    if app.filter_rect.contains((mouse.column, mouse.row).into()) {
                        app.begin_filter();
                    } else if app.table_rect.contains((mouse.column, mouse.row).into()) {
                        let index = app.table_offset + (mouse.row - app.table_rect.y) as usize;
                        if index < app.row_count() {
                            app.selected = index;
                            app.remember_selection();
                            app.editing_filter = false;
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && app.detail.is_none()
            && !app.help
            && !app.settings
            && app.main_menu.is_none()
        {
            if let Some((_, code)) = app
                .controls
                .iter()
                .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
            {
                app.editing_filter = false;
                self.last_click = None;
                self.queued_key = Some(KeyEvent::new(*code, KeyModifiers::NONE));
                return;
            }
            if let Some((_, index)) = app
                .gpu_ranges
                .iter()
                .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
            {
                let index = *index;
                if app.selected_gpu != index {
                    app.selected_gpu = index;
                    app.selected = 0;
                    app.clear_process_selection();
                }
                if self.last_click.is_some_and(|(at, x, y)| {
                    at.elapsed() < Duration::from_millis(400) && x == mouse.column && y == mouse.row
                }) {
                    app.focus_panel(2);
                    self.last_click = None;
                } else {
                    self.last_click = Some((Instant::now(), mouse.column, mouse.row));
                }
                return;
            }
            if let Some((_, code)) = app
                .footer_actions
                .iter()
                .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
            {
                self.queued_key = Some(*code);
                return;
            }
            if app.filter_rect.contains((mouse.column, mouse.row).into()) {
                if !app.editing_filter {
                    app.begin_filter();
                }

                return;
            }
            if let Some((_, key)) = app
                .label_targets
                .iter()
                .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
            {
                let key = key.clone();
                if app
                    .process_detail_rect
                    .contains((mouse.column, mouse.row).into())
                {
                    app.focus_panel(0);
                }
                app.clicked_label = Some(key);
                self.queued_key = Some(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::NONE));
                self.last_click = None;
                return;
            }
            app.editing_filter = false;
        }
        if app.settings {
            if app.settings_rect.contains((mouse.column, mouse.row).into()) {
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    if let Some((_, index, key)) = app
                        .dialog
                        .settings_adjust
                        .iter()
                        .find(|(rect, _, _)| rect.contains((mouse.column, mouse.row).into()))
                    {
                        app.setting_index = *index;
                        self.queued_key = Some(KeyEvent::new(*key, KeyModifiers::NONE));
                    } else if let Some((_, index)) = app
                        .dialog
                        .settings_rows
                        .iter()
                        .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                    {
                        app.setting_index = *index;
                        app.dialog.settings_message.clear();
                    }
                } else if mouse.kind == MouseEventKind::ScrollDown {
                    app.dialog.settings_message.clear();
                    app.setting_index =
                        (app.setting_index + 1).min(rstop::config::Config::SETTINGS_COUNT - 1);
                } else if mouse.kind == MouseEventKind::ScrollUp {
                    app.dialog.settings_message.clear();
                    app.setting_index = app.setting_index.saturating_sub(1);
                }
            }
            return;
        }

        if let Some(menu) = &mut app.main_menu
            && !app.help
            && app.detail.is_none()
        {
            match mouse.kind {
                MouseEventKind::ScrollDown => {
                    menu.selected = (menu.selected + 1) % rstop::app::menu::MenuAction::ALL.len()
                }
                MouseEventKind::ScrollUp => {
                    menu.selected = (menu.selected + rstop::app::menu::MenuAction::ALL.len() - 1)
                        % rstop::app::menu::MenuAction::ALL.len()
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if menu.back_rect.contains((mouse.column, mouse.row).into()) {
                        self.queued_key = Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                    } else if let Some((_, link)) = menu
                        .links
                        .iter()
                        .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                    {
                        self.queued_key = Some(KeyEvent::new(link.key(), KeyModifiers::NONE));
                    } else if let Some((_, index)) = menu
                        .hitboxes
                        .iter()
                        .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                    {
                        menu.selected = *index;
                        self.queued_key = Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    }
                }
                _ => {}
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if app.detail.is_some() || app.help {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_add(3)
                } else if app
                    .process_detail_rect
                    .contains((mouse.column, mouse.row).into())
                {
                    app.scroll_selection(3);
                } else if app.has_process_table() {
                    app.table_offset = (app.table_offset + 3).min(
                        app.row_count()
                            .saturating_sub(app.table_rect.height as usize),
                    );
                } else {
                    app.move_selection(3)
                }
            }
            MouseEventKind::ScrollUp => {
                if app.detail.is_some() || app.help {
                    app.follow_logs = false;
                    app.detail_scroll = app.detail_scroll.saturating_sub(3)
                } else if app
                    .process_detail_rect
                    .contains((mouse.column, mouse.row).into())
                {
                    app.scroll_selection(-3);
                } else if app.has_process_table() {
                    app.table_offset = app.table_offset.saturating_sub(3);
                } else {
                    app.move_selection(-3)
                }
            }
            MouseEventKind::Down(button @ (MouseButton::Left | MouseButton::Right))
                if app.detail.is_none() && !app.help && app.signal.is_none() =>
            {
                if let Some((_, view)) = app
                    .tab_ranges
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                {
                    let view = *view;
                    app.set_view(view);
                } else if let Some((_, index)) = app
                    .network_sort_headers
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                {
                    let index = *index;
                    app.set_network_sort(index, true);
                    self.last_click = None;
                } else if let Some((_, sort)) = app
                    .sort_headers
                    .iter()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                {
                    let sort = *sort;
                    if app.sort == sort {
                        app.reverse = !app.reverse;
                    } else {
                        app.sort = sort;
                        app.reverse = false;
                    }
                    app.resort_processes();
                } else if app.table_rect.contains((mouse.column, mouse.row).into()) {
                    let index = app.table_offset + (mouse.row - app.table_rect.y) as usize;
                    if index >= app.row_count() {
                        app.clear_active_selection();
                        self.last_click = None;
                        return;
                    }
                    app.selected = index;
                    app.remember_selection();
                    if button == MouseButton::Right {
                        self.queued_key = Some(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE));
                    } else if self.last_click.is_some_and(|(at, x, y)| {
                        at.elapsed() < Duration::from_millis(400)
                            && x == mouse.column
                            && y == mouse.row
                    }) {
                        self.queued_key = Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                        self.last_click = None;
                    } else {
                        self.last_click = Some((Instant::now(), mouse.column, mouse.row));
                    }
                } else if app.has_row_selection()
                    && !app
                        .process_detail_rect
                        .contains((mouse.column, mouse.row).into())
                {
                    app.clear_active_selection();
                    self.last_click = None;
                }
            }
            _ => {}
        }
    }
}
