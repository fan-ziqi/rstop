use super::{Palette, controls, dialogs, tags};
use crate::app::App;
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};
use sysinfo::Signal;

pub(super) fn signal(frame: &mut Frame, app: &mut App, p: Palette) {
    app.popup_actions.clear();
    use crate::app::signals;
    let target = app.signal.as_ref().expect("signal dialog");
    let targets = if app.signal_targets.is_empty() {
        std::slice::from_ref(target)
    } else {
        &app.signal_targets
    };
    let supported = sysinfo::SUPPORTED_SIGNALS;
    let selected = supported
        .iter()
        .position(|signal| *signal == target.signal)
        .unwrap_or(0);
    let choice = signals::describe(target.signal);
    let action = match target.signal {
        Signal::Kill => "Kill",
        Signal::Term => "Terminate",
        Signal::Interrupt => "Interrupt",
        Signal::Stop => "Stop",
        Signal::Continue => "Continue",
        _ => "Signal",
    };
    let accent = if target.signal == Signal::Kill {
        p.red
    } else {
        p.mint
    };
    let width = 84.min(frame.area().width.saturating_sub(4));
    let columns = if width.saturating_sub(6) >= 44 { 2 } else { 1 };
    let total_rows = supported.len().div_ceil(columns);
    let target_rows = targets
        .len()
        .clamp(1, if frame.area().height >= 28 { 3 } else { 1 });
    let area = dialogs::popup(
        frame.area(),
        width,
        (total_rows + target_rows + 10).clamp(14, 34) as u16,
    );
    let inner = dialogs::surface(
        frame,
        area,
        &format!("Process signal · {} available", supported.len()),
        p,
        accent,
    );
    app.popup_rect = area;
    app.signal_ui.hits.clear();
    let button_height = if area.height >= 22 { 3 } else { 1 };
    let mut buttons = button_row(area, inner.width);
    for button in &mut buttons {
        button.y = area.bottom() - 1 - button_height;
        button.height = button_height;
    }
    let description_y = buttons[0].y - 2;
    let grid = Rect::new(
        inner.x,
        inner.y + 1 + target_rows as u16 + 1,
        inner.width,
        description_y.saturating_sub(inner.y + 2 + target_rows as u16),
    );
    let rows = grid.height.max(1) as usize;
    let selected_row = selected / columns;
    let max_offset = total_rows.saturating_sub(rows);
    let mut offset = app.signal_ui.offset.min(max_offset);
    if selected_row < offset {
        offset = selected_row;
    }
    if selected_row >= offset + rows {
        offset = selected_row + 1 - rows;
    }
    app.signal_ui.offset = offset;
    app.signal_ui.columns = columns;
    app.signal_ui.rows = rows;
    let target_offset = app
        .signal_ui
        .target_scroll
        .min(targets.len().saturating_sub(target_rows));
    app.signal_ui.target_scroll = target_offset;
    app.signal_ui.targets_rect = Rect::new(inner.x, inner.y + 1, inner.width, target_rows as u16);
    let heading = if targets.len() == 1 {
        format!("{action} process?")
    } else {
        format!(
            "{action} {} processes?   {}–{} / {}",
            targets.len(),
            target_offset + 1,
            (target_offset + target_rows).min(targets.len()),
            targets.len()
        )
    };
    frame.render_widget(
        Paragraph::new(heading).style(Style::new().fg(p.text).bold()),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    for (row, target) in targets
        .iter()
        .skip(target_offset)
        .take(target_rows)
        .enumerate()
    {
        let label = tags::elide(
            &format!("PID {}  ·  {}", target.pid, target.name),
            inner.width,
        );
        frame.render_widget(
            Paragraph::new(label).style(Style::new().fg(p.muted)),
            Rect::new(inner.x, inner.y + 1 + row as u16, inner.width, 1),
        );
    }
    let cell_width = (grid.width.saturating_sub(2 + columns as u16 - 1)) / columns as u16;
    for (index, signal) in supported
        .iter()
        .copied()
        .enumerate()
        .skip(offset * columns)
        .take(rows * columns)
    {
        let row = index / columns - offset;
        let column = index % columns;
        let rect = Rect::new(
            grid.x + column as u16 * (cell_width + 1),
            grid.y + row as u16,
            cell_width,
            1,
        );
        let info = signals::describe(signal);
        let active = index == selected;
        let focused = active;
        let mut spans = vec![
            Span::styled(
                if active { "▸ " } else { "  " },
                Style::new().fg(if focused { accent } else { p.muted }),
            ),
            Span::styled(
                format!("{:<10}", info.name),
                Style::new().fg(if active { p.text } else { p.muted }),
            ),
            Span::styled(
                info.shortcut
                    .map(|key| format!("[{key}]"))
                    .unwrap_or_else(|| "   ".into()),
                Style::new().fg(if active { accent } else { p.blue }),
            ),
        ];
        let remaining = cell_width.saturating_sub(16);
        if remaining >= 10 {
            spans.push(Span::styled(
                format!(
                    " {}",
                    tags::elide(info.description, remaining.saturating_sub(1))
                ),
                Style::new().fg(if active { p.text } else { p.muted }),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(if active {
                p.selection
            } else {
                p.panel
            })),
            rect,
        );
        app.signal_ui.hits.push((rect, signal));
    }
    if total_rows > rows {
        let mut state = super::scrollbar::state(total_rows, rows, offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint))
                .track_style(Style::new().fg(p.border)),
            grid,
            &mut state,
        );
    }
    frame.render_widget(
        Paragraph::new(tags::elide(
            &format!("{} · {}", choice.name, choice.description),
            inner.width,
        ))
        .style(Style::new().fg(accent)),
        Rect::new(inner.x, description_y, inner.width, 1),
    );
    let hint = if targets.len() > target_rows {
        "↑↓←→ Select · Alt+↑↓ Targets"
    } else {
        "↑↓←→ Select · PgUp/Dn Page"
    };
    frame.render_widget(
        Paragraph::new(tags::elide(hint, inner.width.saturating_sub(7)))
            .style(Style::new().fg(p.muted)),
        Rect::new(inner.x, description_y + 1, inner.width.saturating_sub(7), 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{}/{}", selected + 1, supported.len()))
            .alignment(Alignment::Right)
            .style(Style::new().fg(p.muted)),
        Rect::new(inner.right() - 7, description_y + 1, 7, 1),
    );
    confirmation_buttons(frame, app, buttons, p, accent);
}

fn confirmation_buttons(
    frame: &mut Frame,
    app: &mut App,
    buttons: [Rect; 2],
    p: Palette,
    accent: Color,
) {
    for (rect, key, aliases, label, color) in [
        (buttons[0], KeyCode::Char('y'), "y/Enter", "Confirm", accent),
        (buttons[1], KeyCode::Char('n'), "n/Esc", "Cancel", p.blue),
    ] {
        let content = Line::from(vec![
            Span::styled(format!("[{aliases}] "), Style::new().fg(color).bold()),
            Span::styled(label, Style::new().fg(p.text)),
        ]);
        let mut button = Paragraph::new(content)
            .alignment(Alignment::Center)
            .style(Style::new().bg(p.panel));
        if rect.height >= 3 {
            button = button.block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(p.muted)),
            );
        }
        frame.render_widget(button, rect);
        app.popup_actions.push((rect, key));
    }
}

fn button_row(area: Rect, inner_width: u16) -> [Rect; 2] {
    let width = (inner_width.saturating_sub(3) / 2).min(19);
    let x = area.x + (area.width.saturating_sub(width * 2 + 3)) / 2;
    [
        Rect::new(x, area.bottom() - 4, width, 3),
        Rect::new(x + width + 3, area.bottom() - 4, width, 3),
    ]
}

pub(super) fn prompt(frame: &mut Frame, app: &mut App, help: &str, p: Palette) {
    app.popup_actions.clear();
    let (kind, value, target) = app.prompt.as_ref().expect("input dialog");
    let title = match kind.as_str() {
        "nice" => "Process priority",
        "affinity" => "CPU affinity",
        "network target" => "Test connection",
        "PCAP file" => "Open packet capture",
        "folder path" => "Inspect folder",
        "log file" => "Open log file",
        "capture interface" => "Packet capture · 1/4 · Interface",
        "capture filter" => "Packet capture · 2/4 · Filter",
        "capture output" => "Packet capture · 3/4 · Save file",
        other => other,
    };
    let area = dialogs::popup(frame.area(), 72, 14);
    let inner = dialogs::surface(frame, area, title, p, p.mint);
    app.popup_rect = area;
    let subtitle = target
        .as_ref()
        .map(|p| format!("PID {}  ·  {}", p.pid, p.name))
        .unwrap_or_else(|| "Enter a value".into());
    frame.render_widget(
        Paragraph::new(tags::elide(&subtitle, inner.width)).style(Style::new().fg(p.muted)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let input = Rect::new(inner.x, inner.y + 2, inner.width, 3);
    app.dialog.input = input.inner(Margin::new(2, 1));
    let cursor = Line::raw(&value[..app.editor.position(value)]).width() as u16;
    let scroll = cursor.saturating_sub(app.dialog.input.width.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(value.as_str())
            .scroll((0, scroll))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(if app.dialog.error.is_empty() {
                        p.blue
                    } else {
                        p.red
                    }))
                    .padding(Padding::horizontal(1)),
            )
            .style(Style::new().fg(p.text).bg(p.bg)),
        input,
    );
    frame.set_cursor_position((
        input.x
            + 2
            + cursor
                .saturating_sub(scroll)
                .min(app.dialog.input.width.saturating_sub(1)),
        input.y + 1,
    ));
    let error = app
        .dialog
        .error
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>();
    frame.render_widget(
        Paragraph::new(if error.is_empty() { help } else { &error })
            .wrap(Wrap { trim: false })
            .style(Style::new().fg(if error.is_empty() { p.muted } else { p.red })),
        Rect::new(inner.x, inner.y + 6, inner.width, 2),
    );
    let buttons = button_row(area, inner.width);
    let apply = match kind.as_str() {
        "network target" => "Test",
        "PCAP file" | "log file" => "Open",
        "folder path" => "Scan",
        kind if kind.starts_with("capture ") => "Next",
        _ => "Apply",
    };
    for (rect, key, label, active) in [
        (buttons[0], KeyCode::Enter, apply, true),
        (buttons[1], KeyCode::Esc, "Cancel", false),
    ] {
        controls::nav_button(frame, rect, key, label, active, p, p.panel);
        app.popup_actions.push((rect, key));
    }
}

pub(super) fn confirm(frame: &mut Frame, app: &mut App, title: &str, body: &str, p: Palette) {
    app.popup_actions.clear();
    let width = 76.min(frame.area().width.saturating_sub(4));
    let paragraph = Paragraph::new(body).wrap(Wrap { trim: false });
    let rows = paragraph.line_count(width.saturating_sub(6).max(1));
    let area = dialogs::popup(frame.area(), width, (rows + 8).clamp(12, 28) as u16);
    let inner = dialogs::surface(frame, area, title, p, p.amber);
    app.popup_rect = area;
    let content = Rect::new(
        inner.x,
        inner.y,
        inner.width.saturating_sub(1),
        inner.height.saturating_sub(4),
    );
    let rows = paragraph.clone().line_count(content.width.max(1));
    app.dialog.rows = content.height as usize;
    app.dialog.lines = rows;
    app.dialog.content = content;
    app.dialog.scroll = app
        .dialog
        .scroll
        .min(rows.saturating_sub(content.height as usize));
    frame.render_widget(
        paragraph
            .style(Style::new().fg(p.text))
            .scroll((app.dialog.scroll.min(u16::MAX as usize) as u16, 0)),
        content,
    );
    if rows > content.height as usize {
        let mut state = super::scrollbar::state(rows, content.height as usize, app.dialog.scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint)),
            Rect::new(content.right(), content.y, 1, content.height),
            &mut state,
        );
        frame.render_widget(
            Paragraph::new(format!(
                "[↑↓/PgUp/PgDn] Review  {}–{}/{}",
                app.dialog.scroll + 1,
                (app.dialog.scroll + content.height as usize).min(rows),
                rows
            ))
            .style(Style::new().fg(p.muted)),
            Rect::new(inner.x, content.bottom(), inner.width, 1),
        );
    }
    let buttons = button_row(area, inner.width);
    confirmation_buttons(frame, app, buttons, p, p.amber);
}
