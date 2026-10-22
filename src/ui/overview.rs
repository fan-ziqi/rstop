use super::{
    Palette, block,
    charts::{Series, Trend, chart},
    health, meter, optional, optional_bytes,
    processes::process_table,
    resources::storage_view,
    text,
};
use crate::{
    app::{App, is_loopback},
    model::{bytes, percent},
};
use ratatui::prelude::*;

fn gpu_panel_visible(app: &App) -> bool {
    app.config.collect_gpu || app.replay && !app.snapshot.gpus.is_empty()
}

pub(super) fn overview(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    match app.config.display.overview_panel.as_str() {
        "cpu" => {
            cpu(frame, area, app, p);
            return;
        }
        "memory" => {
            memory(frame, area, app, p);
            return;
        }
        "gpu" => {
            gpu_card(frame, area, app, p);
            return;
        }
        "storage" => {
            storage_card(frame, area, app, p);
            return;
        }
        "network" => {
            network_card(frame, area, app, p);
            return;
        }
        _ => {}
    }
    if let Some(layout) = app.config.custom_layout.clone() {
        custom_layout(frame, area, app, p, &layout);
        return;
    }
    if app.config.layout == "compute" {
        let cols = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .spacing(1)
            .split(area);
        cpu(frame, cols[0], app, p);
        if !gpu_panel_visible(app) {
            memory(frame, cols[1], app, p);
        } else {
            gpu_card(frame, cols[1], app, p);
        }
        return;
    }
    if app.config.layout == "storage" {
        storage_view(frame, area, app, p);
        return;
    }

    if area.height < 30 || area.width < 100 || app.config.layout == "minimal" {
        let cols = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
            .spacing(1)
            .split(area);
        cpu(frame, cols[0], app, p);
        memory(frame, cols[1], app, p);
        return;
    }
    // Reserve one row per pair of cores, plus the model, graph heading and borders.
    // Keep enough space below for four memory traces and both network directions.
    let cpu_height = (app.snapshot.cores.len().div_ceil(2) as u16 + 5)
        .max(12)
        .min(area.height.saturating_sub(14));
    let rows = Layout::vertical([
        Constraint::Length(cpu_height.min(area.height - 10)),
        Constraint::Min(10),
    ])
    .spacing(1)
    .split(area);
    cpu(frame, rows[0], app, p);
    let count = if gpu_panel_visible(app) { 4 } else { 3 };
    let cards = Layout::horizontal(vec![Constraint::Ratio(1, count); count as usize])
        .spacing(1)
        .split(rows[1]);
    memory(frame, cards[0], app, p);
    let offset = if gpu_panel_visible(app) { 2 } else { 1 };
    if offset == 2 {
        gpu_card(frame, cards[1], app, p);
    }
    storage_card(frame, cards[offset], app, p);
    network_card(frame, cards[offset + 1], app, p);
}

pub(super) fn layout_visible(node: &crate::config::LayoutNode, app: &App) -> bool {
    use crate::config::LayoutNode::*;
    match node {
        Panel { panel } => match panel.as_str() {
            "gpu" => gpu_panel_visible(app),
            _ => true,
        },
        Split { children, .. } => children.iter().any(|node| layout_visible(node, app)),
    }
}
pub(super) fn custom_layout(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    p: Palette,
    node: &crate::config::LayoutNode,
) {
    use crate::config::LayoutNode::*;
    match node {
        Panel { panel } => match panel.as_str() {
            "cpu" => cpu(frame, area, app, p),
            "memory" => memory(frame, area, app, p),
            "gpu" => gpu_card(frame, area, app, p),
            "storage" => storage_card(frame, area, app, p),
            "network" => network_card(frame, area, app, p),
            "processes" => process_table(frame, area, app, p),
            _ => {}
        },
        Split {
            direction,
            children,
            weights,
        } => {
            let children: Vec<_> = children
                .iter()
                .enumerate()
                .filter(|(_, c)| layout_visible(c, app))
                .map(|(i, c)| (c, weights.get(i).copied().unwrap_or(1) as u32))
                .collect();
            let sum: u32 = children.iter().map(|(_, w)| *w).sum();
            if sum == 0 {
                return;
            }
            let constraints = children
                .iter()
                .map(|(_, w)| Constraint::Ratio(*w, sum))
                .collect::<Vec<_>>();
            let rects = Layout::new(
                if direction == "horizontal" {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                },
                constraints,
            )
            .split(area);
            for ((child, _), rect) in children.iter().zip(rects.iter()) {
                custom_layout(frame, *rect, app, p, child);
            }
        }
    }
}

fn fit(value: &str, width: usize) -> String {
    if Line::raw(value).width() <= width {
        return value.to_string();
    }
    let mut result = String::new();
    for c in value.chars() {
        if Line::raw(format!("{result}{c}…")).width() > width {
            break;
        }
        result.push(c);
    }
    if width > 0 {
        result.push('…');
    }
    result
}

pub(super) fn cpu(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let s = &app.snapshot;
    let mut average = if area.width >= 86 {
        format!(
            " AVG 1m {:.2} · 5m {:.2} · 15m {:.2} · up {} ",
            s.load[0],
            s.load[1],
            s.load[2],
            crate::model::duration(s.uptime)
        )
    } else {
        format!(
            " AVG {:.2} / {:.2} / {:.2} ",
            s.load[0], s.load[1], s.load[2]
        )
    };
    if app.config.display.show_battery
        && area.width >= 100
        && let Some(battery) = s.batteries.first()
    {
        average.push_str(&format!(
            " · {} {}{} ",
            battery.name,
            battery
                .percent
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "—".into()),
            battery
                .watts
                .map(|v| format!(" {v:.1} W"))
                .unwrap_or_default()
        ));
    }
    let b = block(
        format!(" CPU  {:.1}% · {} cores ", s.cpu_percent, s.cores.len()),
        p,
        p.mint,
    )
    .title_bottom(Line::styled(average, Style::new().fg(p.muted)));
    let inner = b.inner(area);
    frame.render_widget(b, area);
    if inner.height < 2 {
        return;
    }
    let temperature = if app.config.display.cpu_sensor.is_empty() {
        cpu_temperature(s)
    } else {
        s.sensors
            .iter()
            .find(|sensor| sensor.name == app.config.display.cpu_sensor)
            .map(|sensor| sensor.temperature)
            .filter(|v| v.is_finite())
    };
    let mut stats = vec![
        Span::styled(
            format!(
                "Clock {}",
                if s.cpu_mhz > 0 {
                    format!("{} MHz", s.cpu_mhz)
                } else {
                    "—".into()
                }
            ),
            Style::new().fg(p.blue),
        ),
        Span::styled(
            format!(
                " · Temp {}",
                temperature
                    .map(|t| app.config.display.temperature(t as f64, 1))
                    .unwrap_or_else(|| "—".into())
            ),
            Style::new().fg(temperature.map_or(p.muted, |t| health(t as f64, p.amber, p))),
        ),
    ];
    if let Some(power) = cpu_power(s) {
        stats.push(Span::styled(
            format!(" · {power:.1} W"),
            Style::new().fg(p.amber),
        ));
    }
    let stats = Line::from(stats);
    let info_width = inner.width.saturating_sub(2);
    let name_width = (info_width as usize).saturating_sub(stats.width() + 3);
    let info_height = if name_width >= 12 { 1 } else { 2 }.min(inner.height - 1);
    let mut info = vec![Line::styled(
        fit(
            &s.cpu_name,
            if info_height == 1 {
                name_width
            } else {
                info_width as usize
            },
        ),
        Style::new().fg(p.text),
    )];
    if info_height == 1 {
        info[0].spans.push(Span::raw(" · "));
        info[0].spans.extend(stats.spans);
    } else {
        info.push(stats);
    }
    text(
        frame,
        Rect::new(inner.x + 1, inner.y, info_width, info_height),
        info,
        p,
    );
    let content = Rect::new(
        inner.x + 1,
        inner.y + info_height,
        inner.width.saturating_sub(2),
        inner.height.saturating_sub(info_height),
    );
    let core_temps = s.core_temperatures.iter().flatten().any(|v| v.is_finite());
    let temperature_width = if core_temps { 7 } else { 0 };
    let label_width = s.cores.len().saturating_sub(1).to_string().len().max(2) + 1;
    let min_cell = label_width + 12 + temperature_width;
    let columns = s.cores.len().clamp(
        1,
        if content.width as usize >= min_cell * 2 {
            2
        } else {
            1
        },
    );
    let min_grid = columns * min_cell;
    let wide = content.width as usize >= min_grid + 26;
    let (history, cores) = if wide {
        let core_width = (columns * (label_width + 21 + temperature_width))
            .max(min_grid)
            .min(content.width as usize - 26);
        let panes =
            Layout::horizontal([Constraint::Min(24), Constraint::Length(core_width as u16)])
                .spacing(2)
                .split(content);
        (panes[0], panes[1])
    } else {
        let grid_slots = s.cores.len().div_ceil(columns);
        let core_height = grid_slots.saturating_add(1);
        if s.cores.is_empty()
            || (content.width as usize) < min_cell
            || core_height + 3 > content.height as usize
        {
            cpu_plots(frame, content, app, p);
            return;
        }
        let panes = Layout::vertical([Constraint::Min(3), Constraint::Length(core_height as u16)])
            .spacing(1)
            .split(content);
        (panes[0], panes[1])
    };
    cpu_plots(frame, history, app, p);
    if s.cores.is_empty() || cores.height < 3 {
        return;
    }
    let rows = s.cores.len().div_ceil(columns).max(1);
    let shown = rows.min(cores.height.saturating_sub(1) as usize);
    text(
        frame,
        Rect::new(cores.x, cores.y, cores.width, 1),
        Line::styled(
            if shown < rows {
                format!(
                    "Per-core history · {}/{}",
                    (shown * columns).min(s.cores.len()),
                    s.cores.len()
                )
            } else {
                "Per-core history".into()
            },
            Style::new().fg(p.muted),
        ),
        p,
    );
    let cell_width = cores.width as usize / columns;
    for (index, value) in s.cores.iter().enumerate() {
        if index % rows >= shown {
            continue;
        }
        let x = cores.x + (index / rows * cell_width) as u16;
        let y = cores.y + 1 + (index % rows) as u16;
        let color = health(*value as f64, p.mint, p);
        text(
            frame,
            Rect::new(x, y, label_width as u16, 1),
            Line::styled(
                format!("C{index:0width$}", width = label_width - 1),
                Style::new().fg(p.muted),
            ),
            p,
        );
        let graph_width = cell_width.saturating_sub(label_width + 9 + temperature_width) as u16;
        if let Some(values) = app.history.series.get(&format!("CPU core {index:03} (%)")) {
            Trend {
                series: &[("Core", values, color)],
                // Keep the top braille dot row clear between adjacent sparklines.
                max: 400. / 3.,
                window: app.plot_window(),
                palette: p,
                axes: false,
                unit: "%",
            }
            .render(
                frame,
                Rect::new(x + label_width as u16 + 1, y, graph_width, 1),
            );
        }
        text(
            frame,
            Rect::new(
                x + cell_width.saturating_sub(7 + temperature_width) as u16,
                y,
                6,
                1,
            ),
            Line::styled(format!("{value:5.1}%"), Style::new().fg(color)),
            p,
        );
        if core_temps {
            let temperature = s
                .core_temperatures
                .get(index)
                .copied()
                .flatten()
                .filter(|v| v.is_finite());
            text(
                frame,
                Rect::new(x + cell_width.saturating_sub(7) as u16, y, 6, 1),
                Line::styled(
                    temperature
                        .map(|v| app.config.display.temperature(v as f64, 0))
                        .unwrap_or_else(|| "—".into()),
                    Style::new().fg(p.amber),
                ),
                p,
            );
        }
    }
}

fn cpu_plots(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let values = |field: &str| {
        let key = format!("CPU {field} (%)");
        if field == "total" {
            Some(&app.history.cpu)
        } else {
            app.history.series.get(&key)
        }
    };
    let upper = app.config.display.cpu_graph.as_str();
    let lower = app.config.display.cpu_graph_lower.as_str();
    super::charts::cpu_chart(
        frame,
        area,
        (upper, values(upper)),
        (lower != "none").then(|| (lower, values(lower))),
        app.plot_window(),
        p,
    );
}

fn cpu_temperature(snapshot: &crate::model::Snapshot) -> Option<f32> {
    snapshot
        .sensors
        .iter()
        .filter_map(|sensor| {
            if !sensor.temperature.is_finite() {
                return None;
            }
            let name = sensor.name.to_ascii_lowercase();
            let rank = if name.contains("cpu package")
                || (name.contains("coretemp") && name.contains("package"))
                || (name.contains("k10temp") && name.contains("tdie"))
            {
                0
            } else if name.contains("k10temp") && name.contains("tctl") {
                1
            } else if name.contains("coretemp")
                || name.starts_with("cpu")
                || name.contains("cpu_thermal")
                || name.contains("cpu-thermal")
                || name.contains("soc_thermal")
            {
                2
            } else {
                return None;
            };
            Some((rank, sensor.temperature))
        })
        .min_by(|(ar, at), (br, bt)| ar.cmp(br).then_with(|| bt.total_cmp(at)))
        .map(|(_, temp)| temp)
}

fn cpu_power(snapshot: &crate::model::Snapshot) -> Option<f64> {
    let packages: Vec<_> = snapshot
        .extra
        .get("CPU power (RAPL)")?
        .as_array()?
        .iter()
        .filter(|entry| {
            entry["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("package"))
        })
        .collect();
    if packages.is_empty() {
        return None;
    }
    packages.iter().try_fold(0., |total, entry| {
        entry["watts"]
            .as_f64()
            .filter(|watts| watts.is_finite() && *watts >= 0.)
            .map(|watts| total + watts)
    })
}

fn plot(frame: &mut Frame, area: Rect, title: &str, trend: Trend<'_>) {
    if area.height < 2 || area.width < 2 {
        return;
    }
    text(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        Line::styled(
            super::charts::title_with_unit(title, trend.unit),
            Style::new().fg(trend.palette.muted),
        ),
        trend.palette,
    );
    trend.render(
        frame,
        Rect::new(area.x, area.y + 1, area.width, area.height - 1),
    );
}

fn history_plots(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    p: Palette,
    series: &[Series<'_>],
    max: Option<f64>,
    unit: &str,
) {
    let scale = |values: &std::collections::VecDeque<f64>| {
        max.unwrap_or_else(|| {
            crate::plot::nice_max(
                values
                    .iter()
                    .copied()
                    .filter(|v| v.is_finite())
                    .fold(1., f64::max),
            )
        })
    };
    if area.height < series.len() as u16 * 3 - 1 {
        chart(
            frame,
            area,
            "History",
            series,
            max.unwrap_or_else(|| {
                series
                    .iter()
                    .map(|(_, values, _)| scale(values))
                    .fold(1., f64::max)
            }),
            unit,
            app.plot_window(),
            p,
        );
        return;
    }
    let rows = Layout::vertical(vec![
        Constraint::Ratio(1, series.len() as u32);
        series.len()
    ])
    .spacing(1)
    .split(area);
    for ((name, values, color), area) in series.iter().zip(rows.iter()) {
        plot(
            frame,
            *area,
            &format!("{name} history"),
            Trend {
                series: &[(*name, values, *color)],
                max: scale(values),
                window: app.plot_window(),
                palette: p,
                axes: true,
                unit,
            },
        );
    }
}

pub(super) fn memory(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let m = &app.snapshot.memory;
    let b = block(" MEMORY ", p, p.purple).title_bottom(Line::styled(
        if area.width >= 30 {
            " Excludes reclaimable cache "
        } else {
            " Cache excluded "
        },
        Style::new().fg(p.muted),
    ));
    let inner = b.inner(area);
    frame.render_widget(b, area);
    if inner.is_empty() {
        return;
    }
    let body = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    text(
        frame,
        Rect::new(body.x, body.y, body.width, 1),
        Line::styled(
            format!("Total {}", bytes(m.total as f64)),
            Style::new().fg(p.text).bold(),
        ),
        p,
    );
    let swap_height = if !app.config.display.show_swap {
        0
    } else if body.height >= 11 {
        2
    } else {
        u16::from(body.height >= 6)
    };
    let graph_height = body.height.saturating_sub(1 + swap_height);
    let rows = Layout::vertical([Constraint::Ratio(1, 4); 4]).split(Rect::new(
        body.x,
        body.y + 1,
        body.width,
        graph_height,
    ));
    for ((name, value, key, color), row) in [
        ("Used", Some(m.used), "RAM (%)", p.purple),
        ("Available", Some(m.available), "RAM available (%)", p.mint),
        ("Cached", m.cached, "RAM cached (%)", p.amber),
        ("Free", m.free, "RAM free (%)", p.blue),
    ]
    .into_iter()
    .zip(rows.iter())
    {
        if row.is_empty() {
            continue;
        }
        let value = value.filter(|_| m.total > 0);
        let amount = optional_bytes(value);
        let ratio = value.map(|v| percent(v, m.total));
        let suffix = ratio
            .map(|v| format!("{v:5.1}%"))
            .unwrap_or_else(|| "—".into());
        let label = format!("{name} (%) {amount}");
        let gap = (row.width as usize).saturating_sub(Line::raw(&label).width() + suffix.len());
        let label = if gap > 0 {
            format!("{label}{}{suffix}", " ".repeat(gap))
        } else {
            format!("{name} (%) {amount}")
        };
        text(
            frame,
            Rect::new(row.x, row.y, row.width, 1),
            Line::styled(
                label,
                Style::new().fg(if value.is_some() { color } else { p.muted }),
            ),
            p,
        );
        if row.height >= 2 {
            let graph = Rect::new(row.x, row.y + 1, row.width, row.height - 1);
            if !app.config.display.memory_graphs {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(Line::from(super::visuals::track(
                        ratio,
                        100.,
                        graph.width as usize,
                        color,
                        p,
                    ))),
                    Rect::new(graph.x, graph.y, graph.width, 1),
                );
            } else if let Some(values) = app.history.series.get(key) {
                Trend {
                    series: &[(name, values, color)],
                    max: 100.,
                    window: app.plot_window(),
                    palette: p,
                    axes: false,
                    unit: "%",
                }
                .render(frame, graph);
            }
        }
    }
    if swap_height > 0 {
        let swap = Rect::new(body.x, body.bottom() - swap_height, body.width, swap_height);
        text(
            frame,
            swap,
            Line::styled(
                format!(
                    "Swap {} / {}",
                    bytes(m.swap_used as f64),
                    bytes(m.swap_total as f64)
                ),
                Style::new().fg(p.muted),
            ),
            p,
        );
        if swap_height == 2 {
            if m.swap_total > 0 {
                if let Some(values) = app.history.series.get("Swap (%)") {
                    Trend {
                        series: &[("Swap", values, p.blue)],
                        max: 100.,
                        window: app.plot_window(),
                        palette: p,
                        axes: false,
                        unit: "%",
                    }
                    .render(
                        frame,
                        Rect::new(swap.x, swap.y + 1, swap.width.saturating_sub(8), 1),
                    );
                }
                text(
                    frame,
                    Rect::new(
                        swap.right() - 7.min(swap.width),
                        swap.y + 1,
                        7.min(swap.width),
                        1,
                    ),
                    Line::styled(
                        format!("{:5.1}%", percent(m.swap_used, m.swap_total)),
                        Style::new().fg(p.blue),
                    ),
                    p,
                );
            } else {
                text(
                    frame,
                    Rect::new(swap.x, swap.y + 1, swap.width, 1),
                    Line::styled("Not configured", Style::new().fg(p.muted)),
                    p,
                );
            }
        }
    }
}

pub(super) fn gpu_card(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let Some(gpu) = app.snapshot.gpus.get(app.selected_gpu) else {
        let title = if app.snapshot.timestamp == 0 && !app.replay {
            " GPU · collecting telemetry "
        } else {
            " GPU · no readable telemetry "
        };
        frame.render_widget(block(title, p, p.blue), area);
        return;
    };
    let sampled = app
        .snapshot
        .extra
        .get("Collection state")
        .and_then(|s| s.get("gpu_processes"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let count = if sampled {
        gpu.processes.len().to_string()
    } else {
        "—".into()
    };
    let gpu_count = app.snapshot.gpus.len();
    let position = format!("{}/{}", app.selected_gpu + 1, gpu_count);
    let previous = " [ Prev ";
    let next = " Next ] ";
    let navigation_width = (previous.len() + position.len() + next.len()) as u16;
    let footer_width = area.width.saturating_sub(2);
    let navigation = gpu_count > 1 && navigation_width <= footer_width && area.height >= 2;
    let metadata_width =
        footer_width.saturating_sub(if navigation { navigation_width } else { 0 }) as usize;
    let full_footer = if gpu_count > 1 {
        format!(" {count} proc · {} ·", gpu.backend)
    } else {
        format!(" {count} processes · {} ", gpu.backend)
    };
    let backend_in_footer = Line::raw(&full_footer).width() <= metadata_width;
    let metadata = if backend_in_footer {
        full_footer
    } else if gpu_count > 1 {
        format!(" {count} proc ·")
    } else {
        format!(" {count} processes ")
    };
    let metadata_in_footer = !navigation || Line::raw(&metadata).width() <= metadata_width;
    let metadata = if metadata_in_footer {
        metadata
    } else {
        String::new()
    };
    let mut footer = Line::styled(metadata, Style::new().fg(p.muted));
    if navigation {
        let x = area.x + 1 + footer.width() as u16;
        let y = area.bottom() - 1;
        let button_style = Style::new().fg(p.blue).bold();
        footer.spans.extend([
            Span::styled(previous, button_style),
            Span::raw(position.clone()),
            Span::styled(next, button_style),
        ]);
        app.controls.extend([
            (
                Rect::new(x, y, previous.len() as u16, 1),
                crossterm::event::KeyCode::Char('['),
            ),
            (
                Rect::new(
                    x + previous.len() as u16 + position.len() as u16,
                    y,
                    next.len() as u16,
                    1,
                ),
                crossterm::event::KeyCode::Char(']'),
            ),
        ]);
    }
    let b = block(format!(" GPU {} · {} ", gpu.id, gpu.vendor), p, p.blue).title_bottom(footer);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let width = inner.width.saturating_sub(13) as usize;
    let vram = gpu
        .memory_used
        .zip(gpu.memory_total)
        .map(|(used, total)| percent(used, total));
    let mut lines = vec![
        Line::styled(
            format!(
                " {}",
                fit(&gpu.name, inner.width.saturating_sub(2) as usize)
            ),
            Style::new().fg(p.text).bold(),
        ),
        if let Some(util) = gpu.utilization {
            meter(" GPU", util as f64, width, p.blue, p)
        } else {
            Line::styled(" GPU   —", Style::new().fg(p.muted))
        },
        Line::raw(""),
        if let Some(util) = vram {
            meter(" VRAM", util, width.saturating_sub(1), p.purple, p)
        } else {
            Line::styled(" VRAM  —", Style::new().fg(p.muted))
        },
        Line::from(format!(
            " {} / {}",
            optional_bytes(gpu.memory_used),
            optional_bytes(gpu.memory_total)
        )),
        Line::from(format!(
            " {} · {} · fan {}",
            gpu.temperature
                .map(|t| app.config.display.temperature(t as f64, 0))
                .unwrap_or_else(|| "—".into()),
            gpu.power_watts
                .map(|w| format!("{w:.0} W"))
                .unwrap_or_else(|| "—".into()),
            optional(gpu.fan, "%")
        )),
    ];
    if !backend_in_footer {
        lines.push(Line::styled(
            if !metadata_in_footer {
                format!(" {count} processes · {}", gpu.backend)
            } else {
                format!(" {}", gpu.backend)
            },
            Style::new().fg(p.muted),
        ));
    }
    let used = lines.len() as u16;
    text(frame, inner, lines, p);
    if inner.height >= used + 3
        && let Some(history) = app.history.gpu_details.get(&gpu.uuid)
    {
        history_plots(
            frame,
            Rect::new(
                inner.x + 1,
                inner.y + used,
                inner.width.saturating_sub(2),
                inner.height - used,
            ),
            app,
            p,
            &[
                ("GPU", &history.utilization, p.blue),
                ("VRAM", &history.vram, p.purple),
            ],
            Some(100.),
            "%",
        );
    }
}

pub(super) fn storage_card(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let b = block(" STORAGE ", p, p.amber);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let mut lines = vec![];
    let read: f64 = app
        .snapshot
        .disk_io
        .iter()
        .filter(|d| d.aggregate)
        .map(|d| d.read_per_sec)
        .sum();
    let write: f64 = app
        .snapshot
        .disk_io
        .iter()
        .filter(|d| d.aggregate)
        .map(|d| d.write_per_sec)
        .sum();
    let read_label = format!(" R {}/s", bytes(read));
    let write_label = format!(" W {}/s", bytes(write));
    let io = format!("{read_label} {write_label}");
    let io_rows = if Line::raw(&io).width() < inner.width as usize {
        1
    } else {
        2
    };
    let shown = app
        .snapshot
        .disks
        .len()
        .min(3)
        .min(inner.height.saturating_sub(io_rows) as usize / 2);
    for disk in app.snapshot.disks.iter().take(shown) {
        let usage = percent(disk.used, disk.total);
        let name_width = (inner.width as usize / 3).clamp(1, 16);
        let name = fit(&disk.mount, name_width);
        let padding = name_width.saturating_sub(Line::raw(&name).width()) + 1;
        let color = health(usage, p.amber, p);
        let mut spans = vec![Span::raw(format!(" {name}{}", " ".repeat(padding)))];
        spans.extend(super::visuals::track(
            (disk.total > 0).then_some(usage),
            100.,
            (inner.width as usize).saturating_sub(name_width + 10),
            color,
            p,
        ));
        spans.push(Span::styled(
            if disk.total > 0 {
                format!(" {usage:5.1}%")
            } else {
                "     — ".into()
            },
            Style::new().fg(color),
        ));
        lines.push(Line::from(spans));
        lines.push(Line::styled(
            format!(
                " {} / {}",
                bytes(disk.used as f64),
                if disk.total > 0 {
                    bytes(disk.total as f64)
                } else {
                    "—".into()
                }
            ),
            Style::new().fg(p.muted),
        ));
    }
    if shown < app.snapshot.disks.len() && inner.height as usize > shown * 2 + io_rows as usize {
        lines.push(Line::styled(
            format!(" +{} filesystems", app.snapshot.disks.len() - shown),
            Style::new().fg(p.muted),
        ));
    } else if app.snapshot.disks.is_empty() && inner.height > 1 {
        lines.push(Line::styled(" No filesystems", Style::new().fg(p.muted)));
    }
    if io_rows == 1 {
        lines.push(Line::styled(io, Style::new().fg(p.muted)));
    } else {
        lines.push(Line::styled(read_label, Style::new().fg(p.muted)));
        lines.push(Line::styled(write_label, Style::new().fg(p.muted)));
    }
    let used = lines.len() as u16;
    text(frame, inner, lines, p);
    if inner.height >= used + 3 {
        let series = [
            ("Read", &app.history.disk_read, p.mint),
            ("Write", &app.history.disk_write, p.amber),
        ];
        history_plots(
            frame,
            Rect::new(
                inner.x + 1,
                inner.y + used,
                inner.width.saturating_sub(2),
                inner.height - used,
            ),
            app,
            p,
            &series,
            None,
            "B/s",
        );
    }
}

pub(super) fn network_card(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let display = &app.config.display;
    let networks: Vec<_> = app
        .snapshot
        .networks
        .iter()
        .filter(|n| {
            if display.network_interface.is_empty() {
                app.config.show_loopback || !is_loopback(&n.name)
            } else {
                n.name == display.network_interface
            }
        })
        .collect();
    let busiest = networks
        .iter()
        .max_by(|a, b| (a.rx_per_sec + a.tx_per_sec).total_cmp(&(b.rx_per_sec + b.tx_per_sec)));
    let footer = busiest
        .map(|n| {
            format!(
                " {} · Top since start ",
                fit(&n.name, area.width.saturating_sub(23) as usize)
            )
        })
        .unwrap_or_default();
    let title = if display.network_interface.is_empty() {
        format!(" NETWORK · {} interfaces ", networks.len())
    } else {
        format!(" NETWORK · {} ", display.network_interface)
    };
    let b = block(title, p, p.mint).title_bottom(Line::styled(footer, Style::new().fg(p.muted)));
    let inner = b.inner(area);
    frame.render_widget(b, area);
    if networks.is_empty() {
        text(
            frame,
            inner,
            if display.network_interface.is_empty() {
                " No interfaces"
            } else {
                " Selected interface unavailable"
            },
            p,
        );
        return;
    }
    let body = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    let rows = if body.height < 12 && body.width >= 56 {
        Layout::horizontal([Constraint::Ratio(1, 2); 2])
            .spacing(1)
            .split(body)
    } else {
        Layout::vertical([Constraint::Ratio(1, 2); 2])
            .spacing(u16::from(body.height >= 15))
            .split(body)
    };
    let empty = crate::history::NetworkHistory::default();
    let h = app.history.network(&app.config).unwrap_or(&empty);
    let mut directions = [
        (
            '↓',
            "Download",
            networks.iter().map(|n| n.rx_per_sec).sum::<f64>(),
            h.rx_peak,
            if display.network_start_zero {
                h.rx_session
            } else {
                networks
                    .iter()
                    .fold(0u64, |sum, n| sum.saturating_add(n.rx_total))
            },
            &h.rx,
            p.mint,
        ),
        (
            '↑',
            "Upload",
            networks.iter().map(|n| n.tx_per_sec).sum::<f64>(),
            h.tx_peak,
            if display.network_start_zero {
                h.tx_session
            } else {
                networks
                    .iter()
                    .fold(0u64, |sum, n| sum.saturating_add(n.tx_total))
            },
            &h.tx,
            p.blue,
        ),
    ];
    if display.network_swap {
        directions.swap(0, 1);
    }
    let factor = if display.network_bits { 8. } else { 1. };
    let mut secondary = display.clone();
    secondary.network_bits = !display.network_bits;
    if secondary.network_bits {
        secondary.network_binary = false;
    }
    for ((arrow, name, rate, top, total, history, color), row) in
        directions.into_iter().zip(rows.iter())
    {
        let rate_text = format!("{arrow} {}", display.rate(rate));
        let other = format!(" ({})", secondary.rate(rate));
        let current = if Line::raw(format!("{rate_text}{other}")).width() <= row.width as usize {
            format!("{rate_text}{other}")
        } else {
            rate_text
        };
        text(
            frame,
            *row,
            vec![
                Line::styled(current, Style::new().fg(color).bold()),
                Line::styled(
                    format!("{arrow} Top {}", display.rate(top)),
                    Style::new().fg(color),
                ),
                Line::styled(
                    format!(
                        "{arrow} {} {}",
                        if display.network_start_zero {
                            "Session"
                        } else {
                            "Total"
                        },
                        bytes(total as f64)
                    ),
                    Style::new().fg(p.muted),
                ),
            ],
            p,
        );
        if row.height >= 5 {
            text(
                frame,
                Rect::new(row.x, row.y + 3, row.width, 1),
                Line::styled(
                    format!(
                        "{name} ({}){}",
                        if display.network_bits { "bit/s" } else { "B/s" },
                        if display.network_logarithmic {
                            " · log"
                        } else {
                            ""
                        }
                    ),
                    Style::new().fg(p.muted),
                ),
                p,
            );
            let max = if display.network_scale > 0 {
                display.network_scale as f64
            } else {
                let max = history
                    .iter()
                    .chain(if display.network_sync_scale {
                        if name == "Download" {
                            h.tx.iter()
                        } else {
                            h.rx.iter()
                        }
                    } else {
                        history.iter()
                    })
                    .copied()
                    .filter(|v| v.is_finite())
                    .fold(1., f64::max);
                crate::plot::nice_max(max)
            };
            let values = history
                .iter()
                .map(|v| v * factor)
                .collect::<std::collections::VecDeque<_>>();
            Trend {
                series: &[(name, &values, color)],
                max: max * factor,
                window: app.plot_window(),
                palette: p,
                axes: true,
                unit: match (display.network_bits, display.network_binary) {
                    (true, true) => "ibit/s",
                    (true, false) => "bit/s",
                    (false, false) => "B/s SI",
                    _ => "B/s",
                },
            }
            .render_scaled(
                frame,
                Rect::new(row.x, row.y + 4, row.width, row.height - 4),
                display.network_logarithmic,
            );
        }
    }
}
