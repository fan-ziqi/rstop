use super::{
    Palette, block, health, optional, optional_bytes,
    visuals::{self, Stat},
};
use crate::{
    app::{App, View},
    model::{bytes, duration},
};
use ratatui::{prelude::*, widgets::*};
pub(super) fn process_summary(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let total = app.snapshot.processes.len();
    let running = app
        .snapshot
        .processes
        .iter()
        .filter(|p| p.state == "Run" || p.state == "Runnable")
        .count();
    visuals::stats(
        frame,
        area,
        &[
            Stat {
                label: "PROCESSES",
                value: total.to_string(),
                color: p.blue,
                percent: None,
            },
            Stat {
                label: "RUNNING",
                value: running.to_string(),
                color: p.mint,
                percent: None,
            },
            Stat {
                label: "HOST CPU",
                value: format!("{:.1}%", app.snapshot.cpu_percent),
                color: health(app.snapshot.cpu_percent as f64, p.mint, p),
                percent: None,
            },
            Stat {
                label: "RAM",
                value: if area.width >= 100 {
                    format!(
                        "{} / {}",
                        bytes(app.snapshot.memory.used as f64),
                        bytes(app.snapshot.memory.total as f64)
                    )
                } else {
                    bytes(app.snapshot.memory.used as f64)
                },
                color: p.purple,
                percent: None,
            },
        ],
        p,
    );
}

pub(super) fn process_table(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let state = app.snapshot.extra.get("Collection state");
    if state
        .and_then(|v| v.get("processes"))
        .and_then(|v| v.as_bool())
        == Some(false)
        || app.view == View::Gpu
            && state
                .and_then(|v| v.get("gpu_processes"))
                .and_then(|v| v.as_bool())
                == Some(false)
    {
        frame.render_widget(
            Paragraph::new("Loading processes…")
                .style(Style::new().fg(p.muted))
                .block(block(" PROCESSES ", p, p.mint)),
            area,
        );
        return;
    }
    if app.process_grouped() {
        program_groups(frame, area, app, p);
        return;
    }
    if app.selected_process().is_some() && area.height >= 9 {
        let (list, detail) = super::selection::split(area);
        if detail.height >= 14 && detail.width >= 40 {
            let panes =
                Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(detail);
            super::selection::draw(frame, panes[0], app, p);
            process_history(frame, panes[1], app, p);
            app.process_detail_rect = detail;
        } else {
            super::selection::draw(frame, detail, app, p);
        }
        table(frame, list, app, p);
    } else {
        app.process_detail_rect = Rect::default();
        table(frame, area, app, p);
    }
}

fn program_groups(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let b = super::controls::selectable_block(" PROGRAM GROUPS ".into(), area, app, p, p.mint)
        .title_bottom(" [Enter] Show members · RSS sum may include shared pages ");
    let inner = b.inner(area);
    let controls = inner;
    let groups = app.program_groups();
    let rows = groups.iter().map(|group| {
        Row::new(vec![
            group.name.clone(),
            group.members.len().to_string(),
            format!("{:.1}%", group.cpu),
            bytes(group.memory as f64),
            format!("{}/s", bytes(group.read_per_sec)),
            format!("{}/s", bytes(group.write_per_sec)),
            optional_bytes(group.gpu_memory),
        ])
    });
    let mut state = TableState::default()
        .with_offset(app.table_offset)
        .with_selected((!groups.is_empty()).then_some(app.selected));
    frame.render_widget(b, area);
    frame.render_stateful_widget(
        Table::new(
            rows,
            [
                Constraint::Min(18),
                Constraint::Length(6),
                Constraint::Length(8),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
            ],
        )
        .header(
            Row::new([
                "PROGRAM", "COUNT", "CPU", "RSS SUM", "READ/s", "WRITE/s", "GPU-MEM",
            ])
            .style(Style::new().fg(p.muted).bold()),
        )
        .column_spacing(1)
        .row_highlight_style(Style::new().bg(p.selection)),
        controls,
        &mut state,
    );
    app.table_rect = Rect::new(
        controls.x,
        controls.y + 1,
        controls.width,
        controls.height.saturating_sub(1),
    );
    app.table_offset = state.offset();
    app.process_detail_rect = Rect::default();
}

fn process_history(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let b = block(" Since selection ", p, p.muted);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let info_height = 0;
    let remaining = inner.height;
    if remaining < 5 {
        return;
    }
    let h = &app.process_history;
    let plots = [
        (
            "CPU · core %",
            &h.cpu,
            p.mint,
            crate::plot::nice_max(h.cpu.iter().copied().fold(100., f64::max)),
            "%",
        ),
        (
            "RAM",
            &h.ram,
            p.purple,
            crate::plot::nice_max(h.ram.iter().copied().fold(1., f64::max)),
            "B",
        ),
        (
            "GPU · process %",
            &h.gpu,
            p.blue,
            crate::plot::nice_max(
                h.gpu
                    .iter()
                    .copied()
                    .filter(|v| v.is_finite())
                    .fold(100., f64::max),
            ),
            "%",
        ),
        (
            "VRAM",
            &h.vram,
            p.purple,
            crate::plot::nice_max(
                h.vram
                    .iter()
                    .copied()
                    .filter(|v| v.is_finite())
                    .fold(1., f64::max),
            ),
            "B",
        ),
    ];
    let indices: &[usize] = if remaining >= 9 {
        &[0, 1, 2, 3]
    } else {
        &[0, 2]
    };
    let grid = Layout::vertical(vec![
        Constraint::Ratio(1, (indices.len() / 2) as u32);
        indices.len() / 2
    ])
    .split(Rect::new(
        inner.x,
        inner.y + info_height,
        inner.width,
        remaining - 1,
    ));
    for (index, &plot_index) in indices.iter().enumerate() {
        let (name, values, color, max, unit) = &plots[plot_index];
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(grid[index / 2]);
        super::charts::gpu_plot(
            frame,
            cols[index % 2],
            name,
            &[(name, values, *color)],
            *max,
            unit,
            app.plot_window(),
            p,
        );
    }
    frame.render_widget(
        Paragraph::new(format!("Since selection · {} samples", h.cpu.len()))
            .style(Style::new().fg(p.muted)),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

fn table(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let rows = app.process_rows();
    let mut title = format!(
        " PROCESSES {}/{} · marked {} ",
        rows.len(),
        app.snapshot.processes.len(),
        app.tagged.len()
    );
    if area.width < 100 {
        title = format!(
            " PROCESSES {}/{} ",
            rows.len(),
            app.snapshot.processes.len()
        );
    }
    if let Some(scope) = &app.object_scope {
        title = format!(" PROCESSES · {} · {} members ", scope.title, rows.len());
    }
    let b = super::controls::selectable_block(title, area, app, p, p.mint);
    let inner = b.inner(area);
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    app.table_offset = app
        .table_offset
        .min(rows.len().saturating_sub(app.table_rect.height as usize));
    let width = |c: &str| match c {
        "pid" => Constraint::Length(8),
        "cpu" | "gpu" | "nice" | "encoder" | "decoder" | "gpu_mem" => Constraint::Length(7),
        "user" => Constraint::Length(10),
        "command" => Constraint::Min(12),
        "tags" => Constraint::Length(super::tags::column_width(inner.width)),
        _ => Constraint::Length(11),
    };
    let columns: Vec<String> = if app.config.columns.is_empty() {
        // Reserve identity before optional metrics and badges, including in the split GPU view.
        let mut c = vec!["pid"];
        if inner.width >= 80 {
            c.push("user");
        }
        c.extend(["cpu", "memory"]);
        if inner.width >= 105 {
            c.extend(["vram", "gpu"]);
        }
        if inner.width >= 170 {
            if app.view == View::Gpu {
                c.extend(["gpu_mem", "runtime", "type"]);
            } else {
                c.extend(["read", "write", "state"]);
            }
        }
        let mut budget = inner
            .width
            .saturating_sub(18 + super::tags::column_width(inner.width));
        c.retain(|column| {
            let cost = match width(column) {
                Constraint::Length(n) => n + 1,
                _ => 13,
            };
            if cost <= budget {
                budget -= cost;
                true
            } else {
                false
            }
        });
        c.extend(["command", "tags"]);
        c.into_iter().map(str::to_string).collect()
    } else {
        let mut c: Vec<_> = app
            .config
            .columns
            .iter()
            .filter(|c| c.as_str() != "tags")
            .cloned()
            .collect();
        if app.config.columns.iter().any(|c| c == "tags") {
            c.push("tags".into());
        }
        c
    };
    app.visible_columns = columns.clone();
    let mut budget = inner.width.saturating_sub(1);
    let default_columns = app.config.columns.is_empty();
    let columns: Vec<_> = columns
        .into_iter()
        .skip(
            app.column_offset
                .min(app.visible_columns.len().saturating_sub(1)),
        )
        .take_while(|column| {
            if default_columns {
                return true;
            }
            let cost = match width(column) {
                Constraint::Length(n) => n + 1,
                _ => 13,
            };
            if cost <= budget {
                budget -= cost;
                true
            } else {
                false
            }
        })
        .collect();
    let widths: Vec<_> = columns.iter().map(|c| width(c)).collect();
    let header_rects = super::tags::columns(inner, &widths, 1);
    for (column, rect) in columns.iter().zip(header_rects.iter()) {
        let sort = crate::app::Sort::for_column(column);
        if let Some(sort) = sort {
            app.sort_headers.push((*rect, sort));
        }
    }
    let offset = app.table_offset;
    let mut badge_plans = vec![];
    let data = rows
        .iter()
        .skip(offset)
        .take(app.table_rect.height as usize)
        .map(|(index, depth)| {
            let process = &app.snapshot.processes[*index];
            let gpu_process = app
                .snapshot
                .gpus
                .get(app.selected_gpu)
                .and_then(|gpu| gpu.processes.iter().find(|p| p.pid == process.pid));
            let gpu_metrics = app.gpu_metrics(process);
            let cells: Vec<_> = columns
                .iter()
                .enumerate()
                .map(|(column_index, column)| {
                    let rect = header_rects[column_index];
                    if column == "tags" {
                        let plan = super::tags::plan(
                            &process.tags,
                            String::new(),
                            rect.width,
                            p,
                            app.config.no_color,
                        );
                        let cell = plan.cell();
                        badge_plans.push(plan);
                        return cell;
                    }
                    let value = match column.as_str() {
                        "pid" => format!(
                            "{}{}",
                            if app.tagged.contains(&(process.pid, process.start_time)) {
                                "●"
                            } else {
                                ""
                            },
                            process.pid
                        ),
                        "user" => process.user.clone(),
                        "cpu" => format!("{:.1}", process.cpu),
                        "memory" => bytes(process.memory as f64),
                        "vms" => bytes(process.virtual_memory as f64),
                        "vram" => optional_bytes(gpu_metrics.0),
                        "gpu" => optional(gpu_metrics.1, ""),
                        "encoder" => optional(gpu_process.and_then(|p| p.encoder), ""),
                        "decoder" => optional(gpu_process.and_then(|p| p.decoder), ""),
                        "gpu_mem" => optional(gpu_process.and_then(|p| p.memory_utilization), ""),
                        "type" => gpu_process
                            .map(|p| p.kind.clone())
                            .unwrap_or_else(|| "—".into()),
                        "read" => bytes(process.read_per_sec),
                        "write" => bytes(process.write_per_sec),
                        "read_total" => bytes(process.read_total as f64),
                        "write_total" => bytes(process.write_total as f64),
                        "rx" => process.rx_per_sec.map(bytes).unwrap_or_else(|| "—".into()),
                        "tx" => process.tx_per_sec.map(bytes).unwrap_or_else(|| "—".into()),
                        "state" => process.state.clone(),
                        "nice" => optional(process.nice, ""),
                        "time" => duration(process.cpu_time_ms / 1000),
                        key if key != "command" => {
                            crate::fields::process_display(process, &app.snapshot, key)
                        }
                        _ => {
                            let command = if app.config.display.process_basename {
                                process.name.clone()
                            } else if process.command.is_empty() {
                                format!("[{}]", process.name)
                            } else {
                                process.command.clone()
                            };
                            format!(
                                "{}{}",
                                if app.tree {
                                    format!(
                                        "{}{} ",
                                        "  ".repeat((*depth).min(12)),
                                        if app.collapsed.contains(&process.pid) {
                                            "▸"
                                        } else {
                                            "╰"
                                        }
                                    )
                                } else {
                                    String::new()
                                },
                                command
                                    .chars()
                                    .skip(app.horizontal_scroll)
                                    .collect::<String>()
                            )
                        }
                    };
                    let metric = match column.as_str() {
                        "cpu" => Some((
                            Some(process.cpu as f64),
                            health(process.cpu as f64, p.mint, p),
                        )),
                        "memory" => Some((Some(process.memory as f64), p.purple)),
                        "gpu" => Some((gpu_metrics.1.map(f64::from), p.blue)),
                        "memory_percent" => Some((
                            (app.snapshot.memory.total > 0).then(|| {
                                crate::model::percent(process.memory, app.snapshot.memory.total)
                            }),
                            p.purple,
                        )),
                        _ => None,
                    };
                    if let Some((number, color)) = metric {
                        return visuals::metric_cell(value, number, color, p);
                    }
                    if column == "state" {
                        return visuals::state_cell(&value, p);
                    }
                    Cell::from(value).style(Style::new().fg(match column.as_str() {
                        "cpu" => health(process.cpu as f64, p.mint, p),
                        "memory" | "vms" => p.purple,
                        "vram" | "gpu" => p.blue,
                        _ => p.text,
                    }))
                })
                .collect();
            Row::new(cells)
        })
        .collect::<Vec<_>>();
    let mut state = TableState::default().with_selected(
        if app.selected_process.is_some()
            && app.selected >= app.table_offset
            && app.selected < app.table_offset + app.table_rect.height as usize
        {
            Some(app.selected - offset)
        } else {
            None
        },
    );
    frame.render_stateful_widget(
        Table::new(data, widths)
            .header(
                Row::new(columns.iter().map(|s| {
                    format!(
                        "{}{}",
                        crate::fields::column_label(s),
                        if crate::app::Sort::for_column(s) == Some(app.sort) {
                            if app.reverse == app.sort.descending() {
                                " ↑"
                            } else {
                                " ↓"
                            }
                        } else {
                            ""
                        }
                    )
                }))
                .style(Style::new().fg(p.muted).bold()),
            )
            .block(b.title_bottom(Line::styled(
                " CPU 100% = 1 core ",
                Style::new().fg(p.muted),
            )))
            .column_spacing(1)
            .row_highlight_style(Style::new().bg(p.selection).bold())
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol("▎"),
        area,
        &mut state,
    );
    app.table_offset = offset + state.offset();
    if let Some(index) = columns.iter().position(|c| c == "tags") {
        super::tags::render_column(
            frame,
            &badge_plans,
            header_rects[index],
            app.table_rect,
            state.offset(),
        );
    }
}
