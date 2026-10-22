//! Complete sampled row data, independently scrollable from the table.
use super::{Palette, block};
use crate::{
    app::{App, View},
    model::bytes,
    tags::Tag,
};
use ratatui::{prelude::*, widgets::*};
use serde_json::Value;

struct Record {
    title: String,
    name: String,
    summary: String,
    command: String,
    tags: Vec<Tag>,
    value: Value,
}
fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect()
}
fn process_value(app: &App, process: &crate::model::Process, key: &str) -> String {
    let gpu = app.gpu_metrics(process);
    let device = app
        .snapshot
        .gpus
        .get(app.selected_gpu)
        .and_then(|g| g.processes.iter().find(|p| p.pid == process.pid));
    match key {
        "pid" => process.pid.to_string(),
        "user" => process.user.clone(),
        "cpu" => format!("{:.1}", process.cpu),
        "memory" => bytes(process.memory as f64),
        "vms" => bytes(process.virtual_memory as f64),
        "vram" => super::optional_bytes(gpu.0),
        "gpu" => super::optional(gpu.1, ""),
        "encoder" => super::optional(device.and_then(|p| p.encoder), ""),
        "decoder" => super::optional(device.and_then(|p| p.decoder), ""),
        "gpu_mem" => super::optional(device.and_then(|p| p.memory_utilization), ""),
        "type" => device.map(|p| p.kind.clone()).unwrap_or_else(|| "—".into()),
        "read" => bytes(process.read_per_sec),
        "write" => bytes(process.write_per_sec),
        "read_total" => bytes(process.read_total as f64),
        "write_total" => bytes(process.write_total as f64),
        "rx" => process.rx_per_sec.map(bytes).unwrap_or_else(|| "—".into()),
        "tx" => process.tx_per_sec.map(bytes).unwrap_or_else(|| "—".into()),
        "state" => process.state.clone(),
        "nice" => super::optional(process.nice, ""),
        "time" => crate::model::duration(process.cpu_time_ms / 1000),
        _ => crate::fields::process_display(process, &app.snapshot, key),
    }
}
fn record(app: &App) -> Option<Record> {
    if app.has_process_table() {
        let p = app.selected_process()?;
        let gpu = app.gpu_metrics(p);
        return Some(Record {
            title: format!("PID {}", p.pid),
            name: p.name.clone(),
            summary: format!(
                "CPU {:.1}% · RAM {}\nGPU {} · VRAM {}",
                p.cpu,
                bytes(p.memory as f64),
                super::optional(gpu.1, "%"),
                super::optional_bytes(gpu.0)
            ),
            command: p.command.clone(),
            tags: p.tags.clone(),
            value: serde_json::to_value(p).ok()?,
        });
    }
    match app.view {
        View::Services => {
            let (view, scope, name) = app.selected_workload.as_ref()?;
            if *view != app.view {
                return None;
            }
            let s = app
                .filtered_services()
                .into_iter()
                .find(|s| &s.scope == scope && &s.name == name)?;
            let command = app
                .snapshot
                .processes
                .iter()
                .find(|p| {
                    Some(p.pid) == s.pid
                        && !p.thread
                        && (app.snapshot.inventory.updated_at == 0
                            || p.start_time <= app.snapshot.inventory.updated_at)
                })
                .map(|p| format!("Main PID {} (sampled)\n{}", p.pid, p.command))
                .unwrap_or_default();
            Some(Record {
                title: "SERVICE".into(),
                name: s.name.clone(),
                summary: format!("{} / {} · {}\n{}", s.active, s.sub, s.scope, s.description),
                command,
                tags: s.tags.clone(),
                value: serde_json::to_value(s).ok()?,
            })
        }
        View::Containers => {
            let (view, runtime, id) = app.selected_workload.as_ref()?;
            if *view != app.view {
                return None;
            }
            let c = app
                .filtered_containers()
                .into_iter()
                .find(|c| &c.runtime == runtime && &c.id == id)?;
            Some(Record {
                title: "CONTAINER".into(),
                name: c.name.clone(),
                summary: format!("{} · {}\nImage: {}", c.runtime, c.state, c.image),
                command: c.command.clone(),
                tags: c.tags.clone(),
                value: serde_json::to_value(c).ok()?,
            })
        }
        View::Network if app.network_section == 0 => {
            let id = app.selected_connection.as_ref()?;
            let c = app
                .filtered_connections()
                .into_iter()
                .find(|c| &c.id == id)?;
            let commands = c
                .pids
                .iter()
                .filter_map(|pid| {
                    app.snapshot.processes.iter().find(|p| {
                        p.pid == *pid
                            && !p.thread
                            && (app.snapshot.connections.owners_sampled_at == 0
                                || p.start_time <= app.snapshot.connections.owners_sampled_at)
                    })
                })
                .map(|p| format!("PID {} · {}\n{}", p.pid, p.name, p.command))
                .collect::<Vec<_>>();
            Some(Record {
                title: "CONNECTION".into(),
                name: c.process_names.join(" / "),
                summary: format!("{} · {}\n{}\n→ {}", c.protocol, c.state, c.local, c.remote),
                command: commands.join("\n\n"),
                tags: c.tags.clone(),
                value: serde_json::to_value(c).ok()?,
            })
        }
        _ => None,
    }
}

struct Content {
    title: String,
    lines: Vec<Line<'static>>,
    tags: Vec<Tag>,
    tag_line: usize,
}
fn content(app: &App, width: u16, p: Palette) -> Option<Content> {
    let mut record = record(app)?;
    let heading = |text: &str| Line::styled(text.to_owned(), Style::new().fg(p.muted));
    let mut lines: Vec<_> = clean(&record.name)
        .lines()
        .map(|s| Line::styled(s.to_owned(), Style::new().fg(p.text)))
        .collect();
    lines.extend(
        clean(&record.summary)
            .lines()
            .map(|s| Line::raw(s.to_owned())),
    );
    lines.extend([Line::raw(""), heading("LABELS · click for actions")]);
    let tag_line = lines.len();
    lines.push(Line::raw(""));
    lines.extend([Line::raw(""), heading("COMMAND")]);
    let command = if record.command.is_empty() {
        "Not present in this sample".into()
    } else {
        clean(&record.command)
    };
    lines.extend(command.lines().map(|s| Line::raw(s.to_owned())));
    lines.push(Line::raw(""));
    if app.has_process_table()
        && let Some(process) = app.selected_process()
    {
        lines.push(heading("TABLE FIELDS"));
        let mut rows = vec![];
        for column in &app.visible_columns {
            if ["command", "tags"].contains(&column.as_str()) {
                continue;
            }
            rows.push((
                crate::fields::column_label(column),
                clean(&process_value(app, process, column)),
            ));
        }
        lines.extend(field_table(&rows, width, p));
        lines.push(Line::raw(""));
    }
    lines.push(heading("ALL SAMPLED FIELDS"));
    if let Some(fields) = record.value.as_object_mut() {
        fields.remove("tags");
        fields.remove("command");
        let mut rows = vec![];
        for (key, value) in fields {
            sampled_fields(key, value, &mut rows);
        }
        lines.extend(field_table(&rows, width, p));
    }
    Some(Content {
        title: record.title,
        lines,
        tags: record.tags,
        tag_line,
    })
}

fn sampled_fields(key: &str, value: &Value, rows: &mut Vec<(String, String)>) {
    if let Value::Object(fields) = value
        && !fields.is_empty()
    {
        for (name, value) in fields {
            sampled_fields(&format!("{key} / {name}"), value, rows);
        }
    } else {
        rows.push((key.to_owned(), clean(&crate::fields::value(value, key))));
    }
}

/// Wrap each cell independently so a continuation stays in its own column.
fn field_table(rows: &[(String, String)], width: u16, p: Palette) -> Vec<Line<'static>> {
    let width = width.max(5) as usize;
    // Both sections use identical column geometry, independent of their keys.
    let key_width = (width * 2 / 5).clamp(1, 24);
    let value_width = width.saturating_sub(key_width + 3).max(1);
    let mut lines = vec![Line::styled(
        format!("{:<key_width$} │ Value", "Field"),
        Style::new().fg(p.muted),
    )];
    lines.push(Line::styled(
        format!("{}─┼─{}", "─".repeat(key_width), "─".repeat(value_width)),
        Style::new().fg(p.border),
    ));
    for (key, value) in rows {
        let keys = wrap_cell(key, key_width as u16);
        let values = wrap_cell(value, value_width as u16);
        for index in 0..keys.len().max(values.len()) {
            let key = keys.get(index).map(String::as_str).unwrap_or("");
            let value = values.get(index).cloned().unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "{key}{}",
                        " ".repeat(key_width.saturating_sub(Line::raw(key).width()))
                    ),
                    Style::new().fg(p.muted),
                ),
                Span::styled(" │ ", Style::new().fg(p.border)),
                Span::raw(value),
            ]));
        }
    }
    lines
}

fn wrap_cell(text: &str, width: u16) -> Vec<String> {
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    let height = paragraph.line_count(width).max(1).min(u16::MAX as usize) as u16;
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    paragraph.render(area, &mut buffer);
    (0..height)
        .map(|y| {
            let mut row = String::new();
            let mut x = 0;
            while x < width {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                x += Line::raw(symbol).width().max(1) as u16;
            }
            row.trim_end().to_owned()
        })
        .collect()
}

/// Wide windows use the right side; compact windows retain a scrollable lower preview.
pub(super) fn split(area: Rect) -> (Rect, Rect) {
    let panes = if area.width >= 120 {
        Layout::horizontal([
            Constraint::Min(60),
            Constraint::Length((area.width * 2 / 5).clamp(42, 90)),
        ])
        .spacing(1)
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Min(4),
            Constraint::Length((area.height / 2).clamp(4, 10)),
        ])
        .split(area)
    };
    (panes[0], panes[1])
}

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    if !app.selection_open {
        return;
    }
    let Some(content) = content(app, area.width.saturating_sub(3), p) else {
        return;
    };
    draw_content(frame, area, app, p, content);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, p: Palette, mut content: Content) {
    let b = block(format!(" {} · Selected ", content.title), p, p.blue).title_bottom(Line::styled(
        " Drag to select · Ctrl+C Copy · Wheel Scroll ",
        Style::new().fg(p.muted),
    ));
    let inner = b.inner(area);
    app.process_detail_rect = area;
    // Keep one cell for the scrollbar and use the same width for wrapping and hits.
    let text_area = Rect::new(
        inner.x,
        inner.y,
        inner.width.saturating_sub(1),
        inner.height,
    );
    let badge_rows = super::tags::full_rows(&content.tags, text_area.width, p, app.config.no_color);
    let badge_start = Paragraph::new(content.lines[..content.tag_line].to_vec())
        .wrap(Wrap { trim: false })
        .line_count(text_area.width);
    let badge_lines: Vec<_> = if badge_rows.is_empty() {
        vec![Line::styled("Unrecognized", Style::new().fg(p.muted))]
    } else {
        badge_rows.iter().map(|row| row.plan.line.clone()).collect()
    };
    content
        .lines
        .splice(content.tag_line..content.tag_line + 1, badge_lines);
    let widget = Paragraph::new(content.lines).wrap(Wrap { trim: false });
    app.selection_lines = widget.line_count(text_area.width);
    app.selection_viewport = inner.height as usize;
    app.scroll_selection(0);
    frame.render_widget(b, area);
    frame.render_widget(
        widget.scroll((app.selection_scroll.min(u16::MAX as usize) as u16, 0)),
        text_area,
    );
    app.text_regions
        .push(crate::app::text_selection::TextRegion::capture(
            format!(
                "selection:{:?}:{:?}:{:?}:{:?}",
                app.view, app.selected_process, app.selected_connection, app.selected_workload
            ),
            (app.selection_scroll, 0),
            text_area,
            frame.buffer_mut(),
        ));
    for (index, row) in badge_rows.iter().enumerate() {
        let line = badge_start + index;
        if line < app.selection_scroll || line >= app.selection_scroll + app.selection_viewport {
            continue;
        }
        let y = text_area.y + (line - app.selection_scroll) as u16;
        for (hit, key) in row.plan.hits(text_area.x, y).into_iter().zip(&row.keys) {
            app.label_hits.push(hit);
            app.label_targets.push((hit, key.clone()));
        }
    }
    if app.selection_lines > app.selection_viewport && inner.height > 0 {
        let mut state = ScrollbarState::new(app.selection_lines)
            .viewport_content_length(app.selection_viewport)
            .position(app.selection_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.muted))
                .track_style(Style::new().fg(p.border)),
            inner,
            &mut state,
        );
    }
}

pub(super) fn with_pane(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) -> Rect {
    if app.selection_open && area.height >= 9 {
        let (table, detail) = split(area);
        if let Some(content) = content(app, detail.width.saturating_sub(3), p) {
            draw_content(frame, detail, app, p, content);
            return table;
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_and_nested_values_start_at_the_left_edge_of_the_same_value_column() {
        let mut rows = vec![];
        sampled_fields("pid", &serde_json::json!(42), &mut rows);
        sampled_fields(
            "extra",
            &serde_json::json!({"long_nested_field": {"count": 17}}),
            &mut rows,
        );
        assert_eq!(
            rows,
            [
                ("pid".into(), "42".into()),
                ("extra / long_nested_field / count".into(), "17".into())
            ]
        );
        let palette = super::super::palette("default");
        for width in [37, 60, 87] {
            let lines = field_table(&rows, width, palette);
            let short = field_table(&[("PID".into(), "42".into())], width, palette);
            let value_column = short[2].to_string().find("42").unwrap();
            for expected in ["42", "17"] {
                let line = lines
                    .iter()
                    .find(|line| line.to_string().contains(expected))
                    .unwrap()
                    .to_string();
                assert_eq!(line.find(expected), Some(value_column));
            }
        }
    }

    #[test]
    fn long_keys_and_unicode_values_keep_separate_columns_at_every_width() {
        let rows = vec![
            ("short".into(), "alpha".into()),
            (
                "a_very_long_sampled_field_name".into(),
                "甲乙👩‍💻e\u{301}abcdefghijklmnopqrstuvwxyz\nsecond line".into(),
            ),
            ("empty".into(), String::new()),
        ];
        for width in [24, 37, 60, 87] {
            let lines = field_table(&rows, width, super::super::palette("default"));
            let mut separator = None;
            let mut values = String::new();
            for line in lines {
                assert!(line.width() <= width as usize, "{width}: {line}");
                let text = line.to_string();
                if let Some((key, value)) = text.split_once(" │ ") {
                    let column = Line::raw(key).width();
                    assert_eq!(*separator.get_or_insert(column), column);
                    values.push_str(value);
                }
            }
            assert!(values.contains("甲乙👩‍💻e\u{301}"), "{values}");
            assert!(values.contains("abcdefghijklmnopqrstuvwxyz"), "{values}");
            assert!(values.contains("second line"), "{values}");
        }
    }
}
