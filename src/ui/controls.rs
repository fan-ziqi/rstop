//! Buttons expose their dispatched key; inline editors never change table geometry.
use super::{Palette, block};
use crate::app::{App, commands::key_name};
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};

pub(super) fn selectable_block<'a>(
    title: String,
    area: Rect,
    app: &mut App,
    p: Palette,
    accent: Color,
) -> Block<'a> {
    let start = (Line::raw(title.as_str()).width() as u16 + 2).min(area.width / 2);
    app.controls_rect = Rect::new(
        area.x + start,
        area.y,
        area.width.saturating_sub(start + 1),
        1,
    );
    block(title, p, accent).border_style(Style::new().fg(if !app.panel_focused {
        p.border
    } else if app.editing_filter
        || matches!(app.view, crate::app::View::Network | crate::app::View::Gpu)
    {
        p.mint
    } else {
        p.blue
    }))
}

pub(super) fn key_spans(key: KeyCode, label: &str, active: bool, p: Palette) -> Vec<Span<'static>> {
    vec![
        Span::raw(" "),
        Span::styled(
            format!("[{}]", key_name(key)),
            Style::new().fg(if active { p.mint } else { p.blue }).bold(),
        ),
        Span::styled(
            format!(" {label} "),
            Style::new().fg(if active { p.mint } else { p.text }),
        ),
    ]
}
pub(super) fn key_button(
    frame: &mut Frame,
    rect: Rect,
    key: KeyCode,
    label: &str,
    active: bool,
    p: Palette,
) {
    frame.render_widget(
        Paragraph::new(Line::from(key_spans(key, label, active, p))),
        rect,
    );
}

pub(super) fn nav_button(
    frame: &mut Frame,
    rect: Rect,
    key: KeyCode,
    label: &str,
    active: bool,
    p: Palette,
    background: Color,
) {
    let style = if active {
        Style::new().fg(p.mint).bg(background).bold()
    } else {
        Style::new().fg(p.text).bg(background)
    };
    let content = Line::from(vec![
        Span::styled(
            format!("[{}]", key_name(key)),
            Style::new().fg(if active { p.mint } else { p.blue }).bold(),
        ),
        Span::styled(
            format!(" {label}"),
            Style::new().fg(if active { p.mint } else { p.text }),
        ),
    ]);
    let mut button = Paragraph::new(content)
        .alignment(Alignment::Center)
        .style(style);
    if rect.height >= 3 {
        button = button.block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(
                    Style::new()
                        .fg(if active { p.mint } else { p.muted })
                        .bg(background),
                ),
        );
    }
    frame.render_widget(button, rect);
}
pub(super) fn button_width(key: KeyCode, label: &str) -> u16 {
    (Line::raw(key_name(key)).width() + Line::raw(label).width() + 5) as u16
}
pub(super) fn table_header(frame: &mut Frame, app: &mut App, p: Palette) {
    let area = app.controls_rect;
    if area.width < 10 || !app.can_filter() {
        return;
    }
    app.filter_rect = Rect::new(area.x, area.y, area.width.saturating_sub(1), 1);
    let prefix = "Filter: ";
    let value = if app.filter.is_empty() && !app.editing_filter {
        match app.view {
            crate::app::View::Network if app.network_section == 0 => "Search connections…",
            crate::app::View::Network => "Search interfaces…",
            crate::app::View::Gpu if app.gpu_section == 2 => "Search device fields…",
            _ => "Search…",
        }
    } else {
        &app.filter
    };
    let cursor_width = Line::raw(&app.filter[..app.editor.position(&app.filter)]).width() as u16;
    let prefix_width = (prefix.len() as u16).min(app.filter_rect.width);
    let available = app.filter_rect.width.saturating_sub(prefix_width + 1);
    let scroll = if app.editing_filter {
        cursor_width.saturating_sub(available.saturating_sub(1))
    } else {
        0
    };
    frame.render_widget(
        Paragraph::new(prefix).style(Style::new().fg(p.muted).bg(p.panel)),
        Rect::new(area.x, area.y, prefix_width, 1),
    );
    frame.render_widget(
        Paragraph::new(value).scroll((0, scroll)).style(
            Style::new()
                .fg(if app.filter.is_empty() && !app.editing_filter {
                    p.muted
                } else {
                    p.text
                })
                .bg(p.panel),
        ),
        Rect::new(
            area.x + prefix_width,
            area.y,
            app.filter_rect.width.saturating_sub(prefix_width),
            1,
        ),
    );
    if app.editing_filter {
        frame.set_cursor_position((
            area.x
                + (prefix_width + cursor_width)
                    .saturating_sub(scroll)
                    .min(app.filter_rect.width.saturating_sub(1)),
            area.y,
        ));
    }
}

pub(super) fn window_label(app: &App) -> String {
    crate::app::commands::time_label(app.config.time_window_seconds)
}

pub(super) fn dim(frame: &mut Frame) {
    let dim_color = |color| match color {
        Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
        other => other,
    };
    for cell in &mut frame.buffer_mut().content {
        cell.fg = dim_color(cell.fg);
        cell.bg = dim_color(cell.bg);
        cell.modifier.remove(Modifier::BOLD);
    }
}
