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
fn content(app: &App, p: Palette) -> Option<Content> {
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
        for column in &app.visible_columns {
            if ["command", "tags"].contains(&column.as_str()) {
                continue;
            }
            lines.push(Line::raw(format!(
                "{}: {}",
                crate::fields::column_label(column),
                clean(&process_value(app, process, column))
            )));
        }
        lines.push(Line::raw(""));
    }
    lines.push(heading("ALL SAMPLED FIELDS"));
    if let Some(fields) = record.value.as_object_mut() {
        fields.remove("tags");
        fields.remove("command");
        for (key, value) in fields {
            let value = if let Some(text) = value.as_str() {
                text.to_string()
            } else {
                crate::fields::pretty(value)
            };
            lines.push(Line::styled(format!("{key}:"), Style::new().fg(p.muted)));
            lines.extend(clean(&value).lines().map(|line| Line::raw(line.to_owned())));
        }
    }
    Some(Content {
        title: record.title,
        lines,
        tags: record.tags,
        tag_line,
    })
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
    let Some(content) = content(app, p) else {
        return;
    };
    draw_content(frame, area, app, p, content);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, p: Palette, mut content: Content) {
    let b = block(format!(" {} · Selected ", content.title), p, p.blue).title_bottom(Line::styled(
        " Wheel / Alt+↑↓ · [d] Full details · [Enter/Esc] Close ",
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
    if app.selection_open
        && area.height >= 9
        && let Some(content) = content(app, p)
    {
        let (table, detail) = split(area);
        draw_content(frame, detail, app, p, content);
        table
    } else {
        area
    }
}
