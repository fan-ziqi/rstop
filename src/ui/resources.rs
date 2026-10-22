use super::{
    Palette, block,
    charts::chart,
    health, optional,
    visuals::{self, Stat, field, metric_cell},
};
use crate::{
    app::App,
    model::{bytes, duration, percent},
};
use ratatui::{prelude::*, widgets::*};

pub(super) fn storage_view(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(if area.height >= 23 {
            area.height / 3
        } else {
            0
        }),
        Constraint::Percentage(35),
        Constraint::Min(4),
    ])
    .split(area);
    let s = &app.snapshot;
    let has_io = s.disk_io.iter().any(|d| d.aggregate);
    let read = s
        .disk_io
        .iter()
        .filter(|d| d.aggregate)
        .map(|d| d.read_per_sec)
        .sum();
    let write = s
        .disk_io
        .iter()
        .filter(|d| d.aggregate)
        .map(|d| d.write_per_sec)
        .sum();
    visuals::stats(
        frame,
        rows[0],
        &[
            Stat {
                label: "FILESYSTEMS",
                value: s.disks.len().to_string(),
                color: p.amber,
                percent: None,
            },
            Stat {
                label: "READ /s",
                value: if has_io { bytes(read) } else { "—".into() },
                color: p.mint,
                percent: None,
            },
            Stat {
                label: "WRITE /s",
                value: if has_io { bytes(write) } else { "—".into() },
                color: p.blue,
                percent: None,
            },
        ],
        p,
    );
    if rows[1].height > 0 {
        let max = crate::plot::nice_max(
            app.history
                .disk_read
                .iter()
                .chain(app.history.disk_write.iter())
                .copied()
                .fold(1024., f64::max)
                * 1.1,
        );
        let b = block(" DISK THROUGHPUT · physical devices ", p, p.amber);
        let inner = b.inner(rows[1]);
        frame.render_widget(b, rows[1]);
        chart(
            frame,
            inner,
            "read / write",
            &[
                ("read", &app.history.disk_read, p.mint),
                ("write", &app.history.disk_write, p.blue),
            ],
            max,
            "B/s",
            app.plot_window(),
            p,
        );
    }
    let wide = area.width >= 110;
    let capacity_width = 8;
    let disk_rows = s
        .disks
        .iter()
        .skip(app.selected.min(s.disks.len().saturating_sub(1)))
        .map(|d| {
            let usage = (d.total > 0).then(|| percent(d.used, d.total));
            let mut cells = vec![Cell::from(d.mount.clone())];
            if wide {
                cells.extend([
                    Cell::from(d.name.clone()),
                    Cell::from(d.filesystem.clone()).style(Style::new().fg(p.muted)),
                ]);
            }
            cells.extend([
                Cell::from(bytes(d.used as f64)),
                Cell::from(bytes(d.total as f64)),
                metric_cell(
                    usage
                        .map(|v| format!("{v:.1}%"))
                        .unwrap_or_else(|| "—".into()),
                    usage,
                    health(usage.unwrap_or(0.), p.amber, p),
                    p,
                ),
            ]);
            Row::new(cells)
        });
    let b = block(" FILESYSTEMS ", p, p.amber);
    let inner = b.inner(rows[2]);
    frame.render_widget(
        Table::new(
            disk_rows,
            if wide {
                vec![
                    Constraint::Min(15),
                    Constraint::Percentage(22),
                    Constraint::Length(8),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(capacity_width as u16),
                ]
            } else {
                vec![
                    Constraint::Min(10),
                    Constraint::Length(11),
                    Constraint::Length(11),
                    Constraint::Length(capacity_width as u16),
                ]
            },
        )
        .header(
            Row::new(if wide {
                vec!["MOUNT", "DEVICE", "FS", "USED", "TOTAL", "CAPACITY"]
            } else {
                vec!["MOUNT", "USED", "TOTAL", "CAPACITY"]
            })
            .style(Style::new().fg(p.muted).bold()),
        )
        .block(b),
        rows[2],
    );
    if s.disks.is_empty() && inner.height > 1 {
        visuals::empty(
            frame,
            Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1),
            "No filesystem data",
            "No readable mounts in this sample",
            p,
        );
    }
    let io_rows = s.disk_io.iter().skip(app.selected).map(|d| {
        let mut cells = vec![
            Cell::from(d.name.clone()),
            Cell::from(format!("{}/s", bytes(d.read_per_sec))).style(Style::new().fg(p.mint)),
            Cell::from(format!("{}/s", bytes(d.write_per_sec))).style(Style::new().fg(p.blue)),
            metric_cell(
                optional(d.busy_percent.map(|v| format!("{v:.1}")), "%"),
                d.busy_percent,
                health(d.busy_percent.unwrap_or(0.), p.amber, p),
                p,
            ),
        ];
        if wide {
            cells.extend([
                Cell::from(optional(d.read_iops.map(|v| format!("{v:.1}")), "")),
                Cell::from(optional(d.write_iops.map(|v| format!("{v:.1}")), "")),
                Cell::from(optional(d.await_ms.map(|v| format!("{v:.2}")), "")),
                Cell::from(optional(d.queue_depth.map(|v| format!("{v:.2}")), "")),
            ]);
        }
        Row::new(cells)
    });
    let mut widths = vec![
        Constraint::Min(8),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(8),
    ];
    let mut headings = vec!["DEVICE", "READ", "WRITE", "BUSY"];
    if wide {
        widths.extend([
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Length(8),
        ]);
        headings.extend(["R IOPS", "W IOPS", "AWAIT ms", "QUEUE"]);
    }
    frame.render_widget(
        Table::new(io_rows, widths)
            .header(Row::new(headings).style(Style::new().fg(p.muted).bold()))
            .block(block(" BLOCK DEVICES ", p, p.amber)),
        rows[3],
    );
}

pub(super) fn network_view(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    super::network::dashboard(frame, area, app, p);
}

pub(super) fn system_view(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let s = &app.snapshot;
    let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
    let mut stats = vec![
        Stat {
            label: "UPTIME",
            value: duration(s.uptime),
            color: p.mint,
            percent: None,
        },
        Stat {
            label: "CORES",
            value: s.cores.len().to_string(),
            color: p.blue,
            percent: None,
        },
        Stat {
            label: "ALERTS",
            value: s.alerts.len().to_string(),
            color: if s.alerts.is_empty() { p.mint } else { p.red },
            percent: None,
        },
    ];
    if let Some(battery) = s.batteries.first() {
        stats.push(Stat {
            label: "BATTERY",
            value: format!(
                "{} · {}",
                optional(battery.percent.map(|v| format!("{v:.0}")), "%"),
                battery.status
            ),
            color: if battery.percent.is_some_and(|v| v < 20.) {
                p.amber
            } else {
                p.mint
            },
            percent: None,
        });
    }
    visuals::stats(frame, sections[0], &stats, p);
    let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    let host_height = if area.width >= 100 { 9 } else { 6 }.min(cols[0].height / 2 + 2);
    let left =
        Layout::vertical([Constraint::Length(host_height), Constraint::Min(3)]).split(cols[0]);
    let right = Layout::vertical([Constraint::Percentage(65), Constraint::Min(3)]).split(cols[1]);
    let mut info = vec![
        field("Host", &s.hostname, p),
        field("OS", &s.os, p),
        field("Kernel", &s.kernel, p),
        field("CPU", &s.cpu_name, p),
        field("Clock", format!("{} MHz", s.cpu_mhz), p),
        field(
            "Sampling",
            format!("{:.2} / {} ms", s.collection_ms, app.config.interval_ms),
            p,
        ),
    ];
    for battery in &s.batteries {
        let mut line = vec![Span::styled(
            format!(" {} ", battery.name),
            Style::new().fg(p.muted),
        )];
        line.extend(visuals::track(
            battery.percent.map(f64::from),
            100.,
            8,
            p.mint,
            p,
        ));
        line.push(Span::styled(
            format!(
                " {} {}",
                optional(battery.percent.map(|v| format!("{v:.0}")), "%"),
                battery.status
            ),
            Style::new().fg(p.text),
        ));
        info.push(Line::from(line));
    }
    frame.render_widget(
        Paragraph::new(info).block(block(" HOST & POWER ", p, p.mint)),
        left[0],
    );
    let b = block(
        format!(
            " TEMPERATURES · {} ",
            app.config.display.temperature_symbol()
        ),
        p,
        p.amber,
    );
    let inner = b.inner(left[1]);
    let sensor_rows = s
        .sensors
        .iter()
        .skip(app.selected.min(s.sensors.len().saturating_sub(1)))
        .map(|sensor| {
            let color = if sensor.critical.is_some_and(|c| sensor.temperature >= c) {
                p.red
            } else if sensor
                .critical
                .is_some_and(|c| sensor.temperature >= c * 0.85)
            {
                p.amber
            } else {
                p.mint
            };
            Row::new(vec![
                Cell::from(sensor.name.clone()),
                metric_cell(
                    format!(
                        "{} / {}",
                        app.config.display.temperature(sensor.temperature as f64, 1),
                        sensor
                            .critical
                            .map(|v| app.config.display.temperature(v as f64, 0))
                            .unwrap_or_else(|| "—".into())
                    ),
                    Some(sensor.temperature as f64),
                    color,
                    p,
                ),
            ])
        });
    frame.render_widget(
        Table::new(sensor_rows, [Constraint::Min(10), Constraint::Length(21)])
            .header(Row::new(["SENSOR", "NOW / CRIT"]).style(Style::new().fg(p.muted).bold()))
            .block(b),
        left[1],
    );
    if s.sensors.is_empty() {
        visuals::empty(
            frame,
            inner,
            "No sensor data",
            "Unavailable on this host",
            p,
        );
    }
    pressure(frame, right[0], app, p);
    let listener_rows = s
        .inventory
        .listeners
        .iter()
        .skip(app.selected)
        .map(|listener| {
            Row::new(vec![
                visuals::state_cell("LISTEN", p),
                Cell::from(format!(
                    "{} {}:{}",
                    listener.protocol, listener.address, listener.port
                )),
                Cell::from(listener.uid.to_string()).style(Style::new().fg(p.muted)),
            ])
        });
    frame.render_widget(
        Table::new(
            listener_rows,
            [
                Constraint::Length(8),
                Constraint::Min(10),
                Constraint::Length(6),
            ],
        )
        .header(Row::new(["STATE", "BOUND ENDPOINT", "UID"]).style(Style::new().fg(p.muted).bold()))
        .block(block(" LISTENING SOCKETS ", p, p.blue)),
        right[1],
    );
}

fn pressure(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    let s = &app.snapshot;
    let b = block(" CPU TIME & PRESSURE ", p, p.purple);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let mut lines = vec![];
    let width = inner.width.saturating_sub(19) as usize;
    if inner.height < 10 {
        let mut spans = vec![Span::styled(
            format!(" CPU {:5.1}% ", s.cpu_percent),
            Style::new().fg(p.mint),
        )];
        spans.extend(visuals::track(
            Some(s.cpu_percent as f64),
            100.,
            inner.width.saturating_sub(13) as usize,
            p.mint,
            p,
        ));
        lines.push(Line::from(spans));
    } else {
        for (key, color) in [
            ("user", p.mint),
            ("system", p.blue),
            ("iowait", p.amber),
            ("steal", p.red),
        ] {
            if let Some(value) = s.cpu_breakdown.get(key) {
                let mut spans = vec![Span::styled(
                    format!(" {key:<7} {value:5.1}% "),
                    Style::new().fg(color),
                )];
                spans.extend(visuals::track(Some(*value), 100., width, color, p));
                lines.push(Line::from(spans));
            }
        }
    }
    lines.push(Line::styled(
        " PSI 10s · some / full",
        Style::new().fg(p.muted).bold(),
    ));
    for pressure in &s.pressure {
        let mut spans = vec![Span::styled(
            format!(" {:6} ", pressure.resource),
            Style::new().fg(p.text),
        )];
        spans.extend(visuals::track(
            pressure.some_10,
            100.,
            width.saturating_sub(7),
            p.purple,
            p,
        ));
        spans.push(Span::styled(
            format!(
                " {} / {}",
                optional(pressure.some_10.map(|v| format!("{v:.1}")), "%"),
                optional(pressure.full_10.map(|v| format!("{v:.1}")), "%")
            ),
            Style::new().fg(p.purple),
        ));
        lines.push(Line::from(spans));
    }
    if s.pressure.is_empty() {
        lines.push(Line::styled(" ◇ PSI unavailable", Style::new().fg(p.muted)));
    }
    lines.push(Line::styled(
        format!(" ◇ {}", s.gpu_status),
        Style::new().fg(p.muted),
    ));
    for note in &s.inventory.notes {
        lines.push(Line::styled(format!(" ◆ {note}"), Style::new().fg(p.amber)));
    }
    for alert in &s.alerts {
        lines.push(Line::styled(format!(" ■ {alert}"), Style::new().fg(p.red)));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
