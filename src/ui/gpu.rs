use super::{
    Palette, block, charts::gpu_plot, meter, optional, optional_bytes, processes::process_table,
    text,
};
use crate::{app::App, model::percent};
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
    let Some(gpu) = app.snapshot.gpus.get(app.selected_gpu) else {
        return;
    };
    let fields = crate::fields::gpu(gpu, &app.filter);
    let grouped = area.width >= 85;
    let rows = fields.iter().map(|(group, name, value)| {
        if grouped {
            Row::new(vec![group.clone(), name.clone(), value.clone()])
        } else {
            Row::new(vec![name.clone(), value.clone()])
        }
    });
    let title = format!(" DEVICE INFO · GPU {} ", gpu.id);
    let b = super::controls::selectable_block(title, area, app, p, p.blue);
    let inner = b.inner(area);
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
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
        .block(b)
        .row_highlight_style(Style::new().bg(p.selection)),
        area,
        &mut state,
    );
    app.table_offset = state.offset();
}
pub(super) fn gpu_content(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    if app.snapshot.gpus.is_empty() {
        let (title, hint) = if !app.config.collect_gpu && !app.replay {
            (
                "GPU collection disabled",
                "GPU queries are turned off for this session",
            )
        } else if app.snapshot.timestamp == 0 && !app.replay {
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
    let card_height = if area.height >= 28 {
        7
    } else if area.height >= 18 {
        5
    } else {
        3
    };
    let rows = Layout::vertical([Constraint::Length(card_height), Constraint::Min(8)]).split(area);
    gpu_cards(frame, rows[0], app, p);
    if area.width >= 130 && rows[1].height >= 18 {
        let columns = Layout::horizontal([Constraint::Percentage(64), Constraint::Percentage(36)])
            .split(rows[1]);
        let left = Layout::vertical([
            Constraint::Length((columns[0].height / 2).clamp(8, 16)),
            Constraint::Min(6),
        ])
        .split(columns[0]);
        gpu_charts(frame, left[0], app, p);
        super::panes::draw(frame, columns[1], 2, app, p, gpu_fields);
        super::panes::draw(frame, left[1], 0, app, p, process_table);
    } else {
        let chart_height = if rows[1].height >= 14 { 4 } else { 0 };
        let info_height = (rows[1].height.saturating_sub(chart_height) / 3).clamp(4, 8);
        let body = Layout::vertical([
            Constraint::Length(chart_height),
            Constraint::Min(4),
            Constraint::Length(info_height),
        ])
        .split(rows[1]);
        if body[0].height > 0 {
            gpu_charts(frame, body[0], app, p);
        }
        super::panes::draw(frame, body[2], 2, app, p, gpu_fields);
        super::panes::draw(frame, body[1], 0, app, p, process_table);
    }
}
fn gpu_cards(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let compact = area.height < 7;
    let visible = if area.width < 100 {
        1
    } else {
        (area.width / 58).clamp(1, 4) as usize
    };
    let start = app.selected_gpu.saturating_sub(visible - 1);
    let count = visible.min(app.snapshot.gpus.len() - start);
    let tiles = Layout::horizontal(vec![Constraint::Ratio(1, count as u32); count])
        .spacing(1)
        .split(area);
    for (offset, tile) in tiles.iter().enumerate() {
        let index = start + offset;
        let gpu = &app.snapshot.gpus[index];
        let selected = index == app.selected_gpu;
        app.gpu_ranges.push((*tile, index));
        let b = block(
            format!(
                " {}GPU {} · {} ",
                if selected { "▸ " } else { "" },
                gpu.id,
                gpu.name
            ),
            p,
            if selected { p.mint } else { p.muted },
        )
        .border_style(Style::new().fg(if selected { p.mint } else { p.border }));
        let inner = b.inner(*tile);
        frame.render_widget(b, *tile);
        if inner.height == 1 {
            text(
                frame,
                inner,
                format!(
                    "GPU {} · VRAM {} / {}",
                    optional(gpu.utilization, "%"),
                    optional_bytes(gpu.memory_used),
                    optional_bytes(gpu.memory_total)
                ),
                p,
            );
            continue;
        }
        let width = inner.width.saturating_sub(14) as usize;
        let used = gpu
            .memory_used
            .zip(gpu.memory_total)
            .filter(|(_, t)| *t > 0)
            .map(|(u, t)| percent(u, t));
        let mut lines = vec![
            gpu.utilization
                .map(|v| meter(" GPU", v as f64, width, p.mint, p))
                .unwrap_or_else(|| {
                    Line::styled(" GPU — · counter unavailable", Style::new().fg(p.muted))
                }),
            used.map(|v| meter("VRAM", v, width, p.purple, p))
                .unwrap_or_else(|| {
                    Line::styled("VRAM — · counter unavailable", Style::new().fg(p.muted))
                }),
            Line::from(format!(
                " {} / {} · {}",
                optional_bytes(gpu.memory_used),
                optional_bytes(gpu.memory_total),
                gpu.performance_state.as_deref().unwrap_or("—")
            )),
        ];
        if !compact {
            lines.push(Line::from(format!(
                " {} · {} / {} · fan {}",
                gpu.temperature
                    .map(|t| app.config.display.temperature(t as f64, 0))
                    .unwrap_or_else(|| "—".into()),
                optional(gpu.power_watts.map(|v| format!("{v:.0}")), "W"),
                optional(gpu.power_limit_watts.map(|v| format!("{v:.0}")), "W"),
                optional(gpu.fan, "%")
            )));
            lines.push(Line::styled(
                format!(
                    " Core {} · MEM {} · {} processes",
                    optional(gpu.clock_mhz, "MHz"),
                    optional(gpu.memory_clock_mhz, "MHz"),
                    gpu.processes.len()
                ),
                Style::new().fg(p.muted),
            ));
        }
        text(frame, inner, lines, p);
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
                format!(
                    "VRAM · {} / {}",
                    optional_bytes(gpu.memory_used),
                    optional_bytes(gpu.memory_total)
                ),
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
            let columns = if area.width >= 130 { 3 } else { 2 };
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
