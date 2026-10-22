use super::{
    Palette, block, optional, optional_bytes,
    visuals::{self, Stat, metric_cell, state_cell},
};
use crate::app::App;
use ratatui::{prelude::*, widgets::*};

fn panes(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::vertical([
        Constraint::Length(if area.height >= 10 { 3 } else { 0 }),
        Constraint::Min(4),
        Constraint::Length(if area.height >= 28 { 8 } else { 0 }),
    ])
    .split(area)
}
fn table_rect(inner: Rect) -> Rect {
    Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    )
}
pub(super) fn containers(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    if app
        .snapshot
        .inventory
        .extra
        .get("Discovery state")
        .and_then(|v| v.get("containers"))
        .and_then(|v| v.as_bool())
        == Some(false)
    {
        frame.render_widget(
            Paragraph::new("Loading local Docker / Podman…")
                .style(Style::new().fg(p.muted))
                .block(block(" WORKLOADS ", p, p.blue)),
            area,
        );
        return;
    }
    let area = super::selection::with_pane(frame, area, app, p);
    let rows = panes(area);
    let inv = &app.snapshot.inventory;
    let running = inv
        .containers
        .iter()
        .filter(|c| c.state == "running")
        .count();
    let unhealthy = inv
        .containers
        .iter()
        .filter(|c| c.status.contains("unhealthy") || c.state == "restarting")
        .count();
    let mut summary = vec![
        Stat {
            label: "CONTAINERS",
            value: inv.containers.len().to_string(),
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
            label: "UNHEALTHY",
            value: unhealthy.to_string(),
            color: if unhealthy > 0 { p.red } else { p.mint },
            percent: None,
        },
    ];
    if area.width >= 130 {
        for (label, value) in [
            (
                "NET RX/s",
                inv.containers
                    .iter()
                    .filter(|c| c.state == "running")
                    .map(|c| c.network_rx_per_sec)
                    .sum::<Option<f64>>(),
            ),
            (
                "NET TX/s",
                inv.containers
                    .iter()
                    .filter(|c| c.state == "running")
                    .map(|c| c.network_tx_per_sec)
                    .sum::<Option<f64>>(),
            ),
            (
                "DISK READ/s",
                inv.containers
                    .iter()
                    .filter(|c| c.state == "running")
                    .map(|c| c.block_read_per_sec)
                    .sum::<Option<f64>>(),
            ),
            (
                "DISK WRITE/s",
                inv.containers
                    .iter()
                    .filter(|c| c.state == "running")
                    .map(|c| c.block_write_per_sec)
                    .sum::<Option<f64>>(),
            ),
        ] {
            summary.push(Stat {
                label,
                value: value
                    .map(|v| format!("{}/s", crate::model::bytes(v)))
                    .unwrap_or_else(|| "—".into()),
                color: p.blue,
                percent: None,
            });
        }
    }
    visuals::stats(frame, rows[0], &summary, p);
    let engine_note = if inv.updated_at == 0 && !app.replay {
        " Discovering local engines… ".into()
    } else {
        format!(
            " {} · sample {}s old ",
            inv.runtimes.join(" + "),
            app.snapshot.timestamp.saturating_sub(inv.updated_at)
        )
    };
    let b = super::controls::selectable_block(" WORKLOADS ".into(), rows[1], app, p, p.blue);
    let inner = b.inner(rows[1]);
    app.table_rect = table_rect(inner);
    let compact = inner.width < 145;
    let detailed_io = inner.width >= 205;
    let mut widths = if compact {
        vec![
            Constraint::Min(17),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(9),
        ]
    } else {
        vec![
            Constraint::Min(26),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Length(11),
            Constraint::Length(6),
            Constraint::Min(18),
        ]
    };
    if detailed_io {
        widths.splice(7..7, [Constraint::Length(11); 4]);
    }
    widths.push(Constraint::Length(super::tags::column_width(inner.width)));
    let columns = super::tags::columns(inner, &widths, 1);
    let containers = app.filtered_containers();
    let is_empty = containers.is_empty();
    let plans: Vec<_> = containers
        .iter()
        .map(|c| {
            super::tags::plan(
                &c.tags,
                String::new(),
                columns.last().unwrap().width,
                p,
                app.config.no_color,
            )
        })
        .collect();
    let data: Vec<_> = containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let values = [
                c.name.clone(),
                c.runtime.clone(),
                c.state.clone(),
                c.cpu
                    .map(|v| format!("{v:.1}%"))
                    .unwrap_or_else(|| "—".into()),
                optional_bytes(c.memory),
                optional_bytes(c.memory_limit),
                optional(c.pids, ""),
                c.network_rx_per_sec
                    .map(|v| format!("{}/s", crate::model::bytes(v)))
                    .unwrap_or_else(|| "—".into()),
                c.network_tx_per_sec
                    .map(|v| format!("{}/s", crate::model::bytes(v)))
                    .unwrap_or_else(|| "—".into()),
                c.block_read_per_sec
                    .map(|v| format!("{}/s", crate::model::bytes(v)))
                    .unwrap_or_else(|| "—".into()),
                c.block_write_per_sec
                    .map(|v| format!("{}/s", crate::model::bytes(v)))
                    .unwrap_or_else(|| "—".into()),
                c.image.clone(),
            ];
            let indices: &[usize] = if compact {
                &[0, 2, 3, 4]
            } else if detailed_io {
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
            } else {
                &[0, 1, 2, 3, 4, 5, 6, 11]
            };
            Row::new(
                indices
                    .iter()
                    .map(|index| {
                        if *index == 0 {
                            Cell::from(super::tags::elide(&values[0], columns[0].width))
                        } else {
                            match *index {
                                2 => state_cell(
                                    if c.status.contains("unhealthy") {
                                        "unhealthy"
                                    } else {
                                        &c.state
                                    },
                                    p,
                                ),
                                3 => metric_cell(values[3].clone(), c.cpu, p.mint, p),
                                4 => metric_cell(
                                    values[4].clone(),
                                    c.memory.map(|v| v as f64),
                                    p.purple,
                                    p,
                                ),
                                _ => Cell::from(values[*index].clone()),
                            }
                        }
                    })
                    .chain(std::iter::once(plans[i].cell())),
            )
            .style(Style::new().fg(
                if c.status.contains("unhealthy") || c.state == "restarting" {
                    p.red
                } else if c.state == "running" {
                    p.text
                } else {
                    p.muted
                },
            ))
        })
        .collect();
    let mut state = TableState::default()
        .with_offset(app.table_offset)
        .with_selected(if is_empty { None } else { Some(app.selected) });
    frame.render_stateful_widget(
        Table::new(data, widths)
            .header(
                Row::new(if compact {
                    vec!["NAME", "STATE", "CPU", "MEMORY", "LABELS"]
                } else if detailed_io {
                    vec![
                        "NAME", "ENGINE", "STATE", "CPU", "MEMORY", "LIMIT", "PIDS", "NET RX/s",
                        "NET TX/s", "READ/s", "WRITE/s", "IMAGE", "LABELS",
                    ]
                } else {
                    vec![
                        "NAME", "ENGINE", "STATE", "CPU", "MEMORY", "LIMIT", "PIDS", "IMAGE",
                        "LABELS",
                    ]
                })
                .style(Style::new().fg(p.muted)),
            )
            .block(b.title_bottom(Line::styled(engine_note, Style::new().fg(p.muted))))
            .column_spacing(1)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(Style::new().bg(p.selection))
            .highlight_symbol("▎"),
        rows[1],
        &mut state,
    );
    app.table_offset = state.offset();
    super::tags::render_column(
        frame,
        &plans,
        *columns.last().unwrap(),
        app.table_rect,
        state.offset(),
    );
    if is_empty {
        let inv = &app.snapshot.inventory;
        let (title, hint) = if !inv.containers.is_empty() {
            ("No matching containers", "Filters apply to this list")
        } else if inv.updated_at == 0 && !app.replay {
            (
                "Loading containers",
                "Discovering local container engines in the background",
            )
        } else if inv.runtimes.is_empty() {
            (
                "No readable container engines",
                "Docker / Podman absent or inaccessible; check System",
            )
        } else {
            ("No containers", "Connected engines have no containers")
        };
        visuals::empty(
            frame,
            Rect::new(
                inner.x,
                inner.y + 1,
                inner.width,
                inner.height.saturating_sub(1),
            ),
            title,
            hint,
            p,
        );
    }
    if rows[2].height > 0 {
        let prefix = app
            .filtered_containers()
            .get(app.selected)
            .map(|c| crate::history::container_key(c));
        workload_history(frame, rows[2], app, p, prefix.as_deref(), "CPU (%)");
    }
}

pub(super) fn services(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    if app
        .snapshot
        .inventory
        .extra
        .get("Discovery state")
        .and_then(|v| v.get("services"))
        .and_then(|v| v.as_bool())
        == Some(false)
    {
        frame.render_widget(
            Paragraph::new("Loading local services…")
                .style(Style::new().fg(p.muted))
                .block(block(" SERVICE UNITS ", p, p.purple)),
            area,
        );
        return;
    }
    let area = super::selection::with_pane(frame, area, app, p);
    let rows = panes(area);
    let inv = &app.snapshot.inventory;
    let failed = inv.services.iter().filter(|s| s.active == "failed").count();
    let active = inv.services.iter().filter(|s| s.active == "active").count();
    visuals::stats(
        frame,
        rows[0],
        &[
            Stat {
                label: "SERVICES",
                value: inv.services.len().to_string(),
                color: p.purple,
                percent: None,
            },
            Stat {
                label: "ACTIVE",
                value: active.to_string(),
                color: p.mint,
                percent: None,
            },
            Stat {
                label: "FAILED",
                value: failed.to_string(),
                color: if failed > 0 { p.red } else { p.mint },
                percent: None,
            },
        ],
        p,
    );
    let manager_note = if inv.updated_at == 0 && !app.replay {
        " Discovering local services… ".into()
    } else {
        format!(" {} ", inv.service_managers.join(" + "))
    };
    let b = super::controls::selectable_block(" SERVICE UNITS ".into(), rows[1], app, p, p.purple);
    let inner = b.inner(rows[1]);
    app.table_rect = table_rect(inner);
    let compact = inner.width < 155;
    let mut widths = if compact {
        vec![
            Constraint::Min(17),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(9),
        ]
    } else {
        vec![
            Constraint::Min(28),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(18),
        ]
    };
    widths.push(Constraint::Length(super::tags::column_width(inner.width)));
    let columns = super::tags::columns(inner, &widths, 0);
    let services = app.filtered_services();
    let empty = services.is_empty();
    let plans: Vec<_> = services
        .iter()
        .map(|s| {
            super::tags::plan(
                &s.tags,
                String::new(),
                columns.last().unwrap().width,
                p,
                app.config.no_color,
            )
        })
        .collect();
    let data: Vec<_> = services
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let values = [
                s.name.clone(),
                s.scope.clone(),
                s.active.clone(),
                s.sub.clone(),
                optional(s.pid, ""),
                s.cpu_percent
                    .map(|v| format!("{v:.1}"))
                    .unwrap_or_else(|| "—".into()),
                optional_bytes(s.memory),
                optional(s.tasks, ""),
                s.description.clone(),
            ];
            let indices: &[usize] = if compact {
                &[0, 2, 5, 6]
            } else {
                &[0, 1, 2, 3, 4, 5, 6, 7, 8]
            };
            Row::new(
                indices
                    .iter()
                    .map(|index| {
                        if *index == 0 {
                            Cell::from(super::tags::elide(&values[0], columns[0].width))
                        } else {
                            match *index {
                                2 => state_cell(&s.active, p),
                                5 => metric_cell(
                                    values[5].clone(),
                                    s.cpu_percent.map(f64::from),
                                    p.mint,
                                    p,
                                ),
                                6 => metric_cell(
                                    values[6].clone(),
                                    s.memory.map(|v| v as f64),
                                    p.purple,
                                    p,
                                ),
                                _ => Cell::from(values[*index].clone()),
                            }
                        }
                    })
                    .chain(std::iter::once(plans[i].cell())),
            )
            .style(Style::new().fg(if s.active == "failed" {
                p.red
            } else if s.active == "active" {
                p.text
            } else {
                p.muted
            }))
        })
        .collect();
    let mut state = TableState::default()
        .with_offset(app.table_offset)
        .with_selected(if empty { None } else { Some(app.selected) });
    frame.render_stateful_widget(
        Table::new(data, widths)
            .header(
                Row::new(if compact {
                    vec!["UNIT", "ACTIVE", "CPU%", "MEMORY", "LABELS"]
                } else {
                    vec![
                        "UNIT",
                        "SCOPE",
                        "ACTIVE",
                        "SUB",
                        "PID",
                        "CPU%",
                        "MEMORY",
                        "TASKS",
                        "DESCRIPTION",
                        "LABELS",
                    ]
                })
                .style(Style::new().fg(p.muted)),
            )
            .block(b.title_bottom(Line::styled(manager_note, Style::new().fg(p.muted))))
            .column_spacing(1)
            .row_highlight_style(Style::new().bg(p.selection)),
        rows[1],
        &mut state,
    );
    app.table_offset = state.offset();
    super::tags::render_column(
        frame,
        &plans,
        *columns.last().unwrap(),
        app.table_rect,
        state.offset(),
    );
    if empty {
        let inv = &app.snapshot.inventory;
        let (title, hint) = if !inv.services.is_empty() {
            ("No matching services", "Filters apply to this list")
        } else if inv.updated_at == 0 && !app.replay {
            (
                "Loading services",
                "Discovering local services in the background",
            )
        } else if inv.service_managers.is_empty() {
            (
                "No readable service manager",
                "Service manager absent or inaccessible; check System",
            )
        } else {
            ("No services", "Connected service managers have no services")
        };
        visuals::empty(frame, app.table_rect, title, hint, p);
    }
    if rows[2].height > 0 {
        let prefix = app
            .filtered_services()
            .get(app.selected)
            .map(|s| format!("Service {}/{}", s.scope, s.name));
        workload_history(frame, rows[2], app, p, prefix.as_deref(), "main CPU (%)");
    }
}

fn workload_history(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    p: Palette,
    prefix: Option<&str>,
    cpu_metric: &str,
) {
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    for (column, metric, label, color) in [
        (columns[0], cpu_metric, "SELECTED CPU HISTORY", p.mint),
        (columns[1], "memory (B)", "SELECTED MEMORY HISTORY", p.blue),
    ] {
        let b = block(format!(" {label} "), p, color);
        let inner = b.inner(column);
        frame.render_widget(b, column);
        let values =
            prefix.and_then(|prefix| app.history.series.get(&format!("{prefix} {metric}")));
        if let Some(values) = values {
            let max = crate::plot::nice_max(
                values
                    .iter()
                    .filter(|v| v.is_finite())
                    .copied()
                    .fold(1., f64::max)
                    * 1.1,
            );
            super::charts::chart(
                frame,
                inner,
                metric,
                &[("sampled", values, color)],
                max,
                app.plot_window(),
                p,
            );
        } else {
            frame.render_widget(
                Paragraph::new("Select a workload; unreported data stays unknown.")
                    .style(Style::new().fg(p.muted)),
                inner,
            );
        }
    }
}
