use super::{
    Palette, block, charts::gpu_plot, meter, optional, optional_bytes, processes::process_table,
    text,
};
use crate::app::App;
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};
use std::collections::VecDeque;
pub(super) fn gpu_view(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    if let Some(gpu) = app.snapshot.gpus.get(app.selected_gpu) {
        let banner = if gpu.vendor == "NVIDIA" {
            format!(
                " Driver {}  ·  CUDA Driver API {}  ·  NVML {}",
                if gpu.driver.is_empty() {
                    "—"
                } else {
                    &gpu.driver
                },
                gpu.cuda_driver_version.as_deref().unwrap_or("—"),
                gpu.nvml_version.as_deref().unwrap_or("—")
            )
        } else {
            format!(
                " Driver {}  ·  Backend {}",
                if gpu.driver.is_empty() {
                    "—"
                } else {
                    &gpu.driver
                },
                gpu.backend
            )
        };
        frame.render_widget(
            Paragraph::new(banner).style(Style::new().fg(p.text)),
            rows[0],
        );
    }
    gpu_content(frame, rows[1], app, p);
}
pub(super) fn gpu_fields(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let title = app
        .snapshot
        .gpus
        .get(app.selected_gpu)
        .map(|gpu| format!("DEVICE INFO · GPU {}", gpu.id))
        .unwrap_or_else(|| "DEVICE INFO".into());
    let inner = super::dialogs::surface(frame, area, &title, p, p.blue);
    app.controls_rect = Rect::new(inner.x, inner.y, inner.width, 1);
    let table = Rect::new(
        inner.x,
        inner.y + 2,
        inner.width.saturating_sub(1),
        inner.height.saturating_sub(4),
    );
    app.table_rect = Rect::new(
        table.x,
        table.y + 1,
        table.width,
        table.height.saturating_sub(1),
    );
    let fields = app
        .snapshot
        .gpus
        .get(app.selected_gpu)
        .map(|gpu| crate::fields::gpu(gpu, &app.filter))
        .unwrap_or_default();
    app.table_offset = app
        .table_offset
        .min(fields.len().saturating_sub(app.table_rect.height as usize));
    let grouped = table.width >= 85;
    let rows = fields.iter().map(|(group, name, value)| {
        if grouped {
            Row::new(vec![group.clone(), name.clone(), value.clone()])
        } else {
            Row::new(vec![name.clone(), value.clone()])
        }
    });
    let mut state = TableState::default()
        .with_selected((!fields.is_empty() && app.panel_focused).then_some(app.selected))
        .with_offset(app.table_offset);
    frame.render_stateful_widget(
        Table::new(
            rows,
            if grouped {
                vec![
                    Constraint::Percentage(22),
                    Constraint::Percentage(35),
                    Constraint::Percentage(43),
                ]
            } else {
                vec![Constraint::Percentage(45), Constraint::Percentage(55)]
            },
        )
        .header(
            Row::new(if grouped {
                vec!["GROUP", "FIELD", "VALUE"]
            } else {
                vec!["FIELD", "VALUE"]
            })
            .style(Style::new().fg(p.muted)),
        )
        .row_highlight_style(Style::new().bg(p.selection)),
        table,
        &mut state,
    );
    app.table_offset = state.offset();
    if fields.is_empty() {
        frame.render_widget(
            Paragraph::new(if app.filter.is_empty() {
                "No device data available"
            } else {
                "No matching fields"
            })
            .style(Style::new().fg(p.muted)),
            app.table_rect,
        );
    }
    if fields.len() > app.table_rect.height as usize {
        let mut scroll = super::scrollbar::state(
            fields.len(),
            app.table_rect.height as usize,
            app.table_offset,
        );
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint))
                .track_style(Style::new().fg(p.border)),
            Rect::new(table.right(), app.table_rect.y, 1, app.table_rect.height),
            &mut scroll,
        );
    }
    let status = if fields.is_empty() {
        "0 fields".into()
    } else {
        format!(
            "{}–{} / {} fields · [↑↓/PgUp/PgDn] Scroll",
            app.table_offset + 1,
            (app.table_offset + app.table_rect.height as usize).min(fields.len()),
            fields.len()
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(Style::new().fg(p.muted)),
        Rect::new(inner.x, inner.bottom() - 2, inner.width, 1),
    );
    app.popup_actions.clear();
    let mut x = inner.x;
    for (key, label, width) in [
        (KeyCode::Esc, "Back", 13),
        (KeyCode::Char('/'), "Find", 12),
        (KeyCode::Enter, "Inspect", 19),
    ] {
        let rect = Rect::new(x, inner.bottom() - 1, width, 1);
        if rect.right() > inner.right() {
            break;
        }
        super::controls::nav_button(frame, rect, key, label, false, p, p.panel);
        app.popup_actions.push((rect, key));
        x += width;
    }
}
pub(super) fn device_info(frame: &mut Frame, app: &mut App, p: Palette) {
    let area = super::dialogs::popup(frame.area(), 110, 32);
    super::panes::draw(frame, area, 2, app, p, gpu_fields);
    app.popup_rect = area;
}
/// `used / capacity`; on unified memory the capacity is the GPU working-set limit and the pool is
/// shared with system RAM, so say so instead of presenting it as separate video memory.
pub(super) fn gpu_memory_text(gpu: &crate::model::Gpu, width: usize) -> String {
    let used = optional_bytes(gpu.memory_used);
    let capacity = optional_bytes(gpu.memory_capacity());
    if !gpu.unified_memory {
        return format!("{used} / {capacity}");
    }
    let long = format!("{used} / {capacity} limit · shared with RAM");
    if long.chars().count() < width {
        long
    } else {
        format!("{used} / {capacity} limit")
    }
}

pub(super) fn gpu_content(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    if app.snapshot.gpus.is_empty() {
        let (title, hint) = if !app.config.collect_gpu && !app.replay {
            (
                "GPU collection disabled",
                "GPU queries are turned off for this session",
            )
        } else if !app.replay
            && (app.snapshot.timestamp == 0
                || app
                    .snapshot
                    .inventory
                    .extra
                    .get("Discovery state")
                    .and_then(|s| s.get("platform_gpu"))
                    .and_then(serde_json::Value::as_bool)
                    == Some(false))
        {
            (
                "Loading GPU telemetry",
                "Collecting device data in the background",
            )
        } else {
            (
                "No readable GPU telemetry",
                app.snapshot.gpu_status.as_str(),
            )
        };
        super::visuals::empty(frame, area, title, hint, p);
        return;
    }
    let card_height = if area.height >= 24 { 4 } else { 3 };
    // Taller windows can show more devices while retaining charts and process rows.
    let card_budget = area
        .height
        .saturating_sub(28)
        .max(if area.height >= 24 { 8 } else { 3 });
    let visible = (card_budget / card_height).max(1) as usize;
    let cards_height = card_height * visible.min(app.snapshot.gpus.len()) as u16;
    let body_height = area.height.saturating_sub(cards_height);
    let process_min = if area.height >= 36 {
        14
    } else if app.selection_open {
        9
    } else {
        5
    };
    let chart_height = if body_height >= process_min + 6 {
        (body_height * 3 / 5)
            .clamp(6, 24)
            .min(body_height - process_min)
    } else {
        0
    };
    let rows = Layout::vertical([
        Constraint::Length(cards_height),
        Constraint::Length(chart_height),
        Constraint::Min(4),
    ])
    .split(area);
    gpu_cards(frame, rows[0], app, p, card_height);
    if rows[1].height > 0 {
        gpu_charts(frame, rows[1], app, p);
    }
    super::panes::draw(frame, rows[2], 0, app, p, process_table);
}
fn gpu_cards(frame: &mut Frame, area: Rect, app: &mut App, p: Palette, card_height: u16) {
    let visible = (area.height / card_height).max(1) as usize;
    let mut start = app
        .gpu_card_offset
        .min(app.snapshot.gpus.len().saturating_sub(visible));
    if app.selected_gpu < start {
        start = app.selected_gpu;
    } else if app.selected_gpu >= start + visible {
        start = app.selected_gpu + 1 - visible;
    }
    app.gpu_card_offset = start;
    let count = visible.min(app.snapshot.gpus.len() - start);
    let tiles = Layout::vertical(vec![Constraint::Length(card_height); count]).split(area);
    for (offset, tile) in tiles.iter().enumerate() {
        let index = start + offset;
        let gpu = &app.snapshot.gpus[index];
        let selected = index == app.selected_gpu;
        app.gpu_ranges.push((*tile, index));
        let title = format!(
            " {}GPU {} · {}{} ",
            if selected { "▸ " } else { "" },
            gpu.id,
            gpu.name,
            if selected && count < app.snapshot.gpus.len() {
                format!(" · {}/{}", index + 1, app.snapshot.gpus.len())
            } else {
                String::new()
            }
        );
        let b = block(
            super::tags::elide(
                &title,
                tile.width.saturating_sub(if selected { 23 } else { 2 }),
            ),
            p,
            if selected { p.mint } else { p.muted },
        )
        .border_style(Style::new().fg(if selected { p.mint } else { p.border }));
        let inner = b.inner(*tile);
        frame.render_widget(b, *tile);
        if selected {
            let info = Rect::new(tile.right() - 21, tile.y, 19, 1);
            super::controls::nav_button(
                frame,
                info,
                KeyCode::Char('I'),
                "Device info",
                false,
                p,
                p.selection,
            );
            app.controls.push((info, KeyCode::Char('I')));
        }
        if inner.height == 1 || inner.width < 74 {
            text(
                frame,
                inner,
                vec![
                    Line::from(format!(
                        "GPU {} · {} {}",
                        optional(gpu.utilization, "%"),
                        gpu.memory_label(),
                        gpu_memory_text(gpu, 0)
                    )),
                    Line::from(format!(
                        "{} · {} · fan {}",
                        gpu.temperature
                            .map(|t| app.config.display.temperature(t as f64, 0))
                            .unwrap_or_else(|| "—".into()),
                        optional(gpu.power_watts.map(|v| format!("{v:.0}")), "W"),
                        optional(gpu.fan, "%")
                    )),
                ],
                p,
            );
            continue;
        }
        let columns = Layout::horizontal(if inner.width >= 110 {
            vec![
                Constraint::Percentage(34),
                Constraint::Percentage(36),
                Constraint::Percentage(30),
            ]
        } else {
            vec![Constraint::Percentage(50), Constraint::Percentage(50)]
        })
        .spacing(1)
        .split(inner);
        let used = gpu.memory_percent();
        let load = vec![
            gpu.utilization
                .map(|v| {
                    meter(
                        " GPU",
                        v as f64,
                        columns[0].width.saturating_sub(14) as usize,
                        p.mint,
                        p,
                    )
                })
                .unwrap_or_else(|| {
                    Line::styled(" GPU — · counter unavailable", Style::new().fg(p.muted))
                }),
            Line::from(format!(
                " {} · {} / {}",
                gpu.temperature
                    .map(|t| app.config.display.temperature(t as f64, 0))
                    .unwrap_or_else(|| "—".into()),
                optional(gpu.power_watts.map(|v| format!("{v:.0}")), "W"),
                optional(gpu.power_limit_watts.map(|v| format!("{v:.0}")), "W"),
            )),
        ];
        let memory = vec![
            used.map(|v| {
                meter(
                    gpu.memory_label(),
                    v,
                    columns[1].width.saturating_sub(14) as usize,
                    p.purple,
                    p,
                )
            })
            .unwrap_or_else(|| {
                Line::styled(
                    format!("{} — · counter unavailable", gpu.memory_label()),
                    Style::new().fg(p.muted),
                )
            }),
            Line::from(format!(
                " {}",
                gpu_memory_text(gpu, columns[1].width as usize)
            )),
        ];
        text(frame, columns[0], load, p);
        text(frame, columns[1], memory, p);
        if columns.len() == 3 {
            text(
                frame,
                columns[2],
                vec![
                    Line::from(if gpu.unified_memory {
                        "No per-process GPU data (macOS)".to_string()
                    } else {
                        format!(
                            "{} · fan {} · {} processes",
                            gpu.performance_state.as_deref().unwrap_or("—"),
                            optional(gpu.fan, "%"),
                            gpu.processes.len()
                        )
                    }),
                    Line::styled(
                        format!(
                            "Core {} · MEM {}",
                            optional(gpu.clock_mhz, "MHz"),
                            optional(gpu.memory_clock_mhz, "MHz"),
                        ),
                        Style::new().fg(p.muted),
                    ),
                ],
                p,
            );
        }
    }
}
fn gpu_charts(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let gpu = &app.snapshot.gpus[app.selected_gpu];
    let compact = area.height < 6;
    if let Some(h) = app.history.gpu_details.get(&gpu.uuid) {
        let known = |values: &VecDeque<f64>| values.iter().any(|v| v.is_finite());
        type Plot<'a> = (
            String,
            Vec<(&'a str, &'a VecDeque<f64>, Color)>,
            f64,
            &'a str,
        );
        let temperature_history = h
            .temperature
            .iter()
            .map(|v| app.config.display.temperature_value(*v))
            .collect::<std::collections::VecDeque<_>>();
        let mut plots: Vec<Plot<'_>> = vec![
            (
                format!("GPU LOAD · {}", optional(gpu.utilization, "%")),
                vec![("GPU", &h.utilization, p.mint)],
                100.,
                "%",
            ),
            (
                format!("{} · {}", gpu.memory_label(), gpu_memory_text(gpu, 0)),
                vec![("used", &h.vram, p.purple)],
                100.,
                "%",
            ),
        ];
        if known(&h.encoder) || known(&h.decoder) || known(&h.memory_busy) {
            plots.push((
                "ENGINES · ENC / DEC / MEM".into(),
                vec![
                    ("ENC", &h.encoder, p.amber),
                    ("DEC", &h.decoder, p.blue),
                    ("MEM", &h.memory_busy, p.purple),
                ],
                100.,
                "%",
            ));
        }
        if known(&h.power) {
            plots.push((
                format!(
                    "POWER · {}",
                    optional(gpu.power_watts.map(|v| format!("{v:.0}")), " W")
                ),
                vec![("power", &h.power, p.amber)],
                gpu.power_limit_watts.map(|v| v as f64).unwrap_or_else(|| {
                    h.power
                        .iter()
                        .copied()
                        .filter(|v| v.is_finite())
                        .fold(1., f64::max)
                        * 1.2
                }),
                "W",
            ));
        }
        if known(&h.temperature) {
            plots.push((
                format!(
                    "TEMPERATURE · {}",
                    gpu.temperature
                        .map(|t| app.config.display.temperature(t as f64, 0))
                        .unwrap_or_else(|| "—".into())
                ),
                vec![("temp", &temperature_history, p.red)],
                app.config.display.temperature_value(110.),
                app.config.display.temperature_symbol(),
            ));
        }
        if known(&h.pcie_rx) || known(&h.pcie_tx) {
            plots.push((
                "PCIE · RX / TX".into(),
                vec![("RX", &h.pcie_rx, p.mint), ("TX", &h.pcie_tx, p.blue)],
                crate::plot::nice_max(
                    h.pcie_rx
                        .iter()
                        .chain(h.pcie_tx.iter())
                        .copied()
                        .filter(|v| v.is_finite())
                        .fold(1024., f64::max)
                        * 1.2,
                ),
                "B/s",
            ));
        }
        if known(&h.fan) {
            plots.push((
                format!("FAN · {}", optional(gpu.fan, "%")),
                vec![("fan", &h.fan, p.mint)],
                100.,
                "%",
            ));
        }
        if known(&h.clock) || known(&h.memory_clock) {
            plots.push((
                "CLOCKS · core / memory".into(),
                vec![
                    ("core", &h.clock, p.blue),
                    ("memory", &h.memory_clock, p.purple),
                ],
                crate::plot::nice_max(
                    h.clock
                        .iter()
                        .chain(h.memory_clock.iter())
                        .copied()
                        .filter(|v| v.is_finite())
                        .fold(1., f64::max),
                ),
                "MHz",
            ));
        }
        if compact {
            gpu_plot(
                frame,
                area,
                &plots[0].0,
                &plots[0].1,
                100.,
                "%",
                app.plot_window(),
                p,
            );
        } else {
            let columns = if area.width >= 150 {
                4
            } else if area.width >= 100 {
                3
            } else if area.width >= 70 {
                2
            } else {
                1
            };
            // Fewer plots on short screens beats squeezing every curve into a line.
            // The remaining counters stay available in the device fields panel.
            plots.truncate(columns * (area.height as usize / 6).max(1));
            let rows_count = plots.len().div_ceil(columns);
            let plot_rows =
                Layout::vertical(vec![Constraint::Ratio(1, rows_count as u32); rows_count])
                    .split(area);
            for (row, chunk) in plots.chunks(columns).enumerate() {
                let cells =
                    Layout::horizontal(vec![Constraint::Ratio(1, chunk.len() as u32); chunk.len()])
                        .spacing(1)
                        .split(plot_rows[row]);
                for (cell, (title, series, max, unit)) in cells.iter().zip(chunk) {
                    gpu_plot(
                        frame,
                        *cell,
                        title,
                        series,
                        *max,
                        unit,
                        app.plot_window(),
                        p,
                    );
                }
            }
        }
    }
}
