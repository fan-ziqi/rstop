use super::{Palette, block, optional, optional_bytes, visuals};
use crate::{
    app::App,
    network_sort::{ConnectionSort as C, InterfaceSort as I},
};
use ratatui::{prelude::*, widgets::*};

fn connection_columns(width: u16, sort: C) -> Vec<C> {
    let mut columns = if width >= 125 {
        vec![
            C::Process,
            C::Pid,
            C::Protocol,
            C::Remote,
            C::State,
            C::Download,
            C::Upload,
            C::Rtt,
            C::Local,
        ]
    } else if width >= 100 {
        vec![
            C::Process,
            C::Pid,
            C::Protocol,
            C::Remote,
            C::State,
            C::Download,
            C::Upload,
        ]
    } else if width >= 86 {
        vec![C::Process, C::Pid, C::Remote, C::Download, C::Upload]
    } else {
        vec![C::Process, C::Pid, C::Remote, C::Traffic]
    };
    if !columns.contains(&sort) {
        if width < 125 {
            columns.pop();
        }
        columns.push(sort);
    }
    columns
}
fn connection_width(column: C, width: u16) -> Constraint {
    match column {
        C::Process => Constraint::Min(if width < 66 { 12 } else { 18 }),
        C::Pid => Constraint::Length(if width < 66 { 6 } else { 7 }),
        C::Protocol => Constraint::Length(5),
        C::Remote | C::Local => Constraint::Min(if width < 66 { 14 } else { 20 }),
        C::State => Constraint::Length(13),
        C::Rtt | C::LocalPort | C::RemotePort => Constraint::Length(9),
        _ => Constraint::Length(if width < 66 { 8 } else { 10 }),
    }
}
fn header_regions(
    app: &mut App,
    area: Rect,
    widths: &[Constraint],
    indices: impl Iterator<Item = usize>,
) {
    let cells = super::tags::columns(area, widths, 1);
    app.network_sort_headers
        .extend(cells.iter().copied().zip(indices));
}
fn heading(label: &str, active: bool, descending: bool, p: Palette) -> Cell<'static> {
    Cell::from(format!(
        "{label}{}",
        if active {
            if descending { " ↓" } else { " ↑" }
        } else {
            ""
        }
    ))
    .style(
        Style::new()
            .fg(if active { p.mint } else { p.muted })
            .bold(),
    )
}
pub(super) fn dashboard(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let focus = app.network_section;
    let editing = app.editing_filter;
    app.set_network_section(0);
    let area = if area.width >= 120 || area.height >= 18 {
        super::selection::with_pane(frame, area, app, p)
    } else if app.selection_open && app.selected_connection.is_some() {
        let panes = Layout::vertical([Constraint::Min(9), Constraint::Length(4)]).split(area);
        super::selection::draw(frame, panes[1], app, p);
        panes[0]
    } else {
        area
    };
    app.set_network_section(focus);
    app.editing_filter = editing;
    let status_height = if area.height >= 24 { 3 } else { 0 };
    let interface_height = if area.height >= 28 {
        8
    } else if area.height >= 17 {
        6
    } else {
        4
    };
    let parts = Layout::vertical([
        Constraint::Length(status_height),
        Constraint::Length(interface_height),
        Constraint::Min(5),
    ])
    .split(area);
    let sample = &app.snapshot.connections;
    let info = if sample.sampled_at == 0 {
        "Discovering sockets…".into()
    } else {
        format!(
            "{} sockets · {} · sample {}s old · owners {}s old{}{}",
            sample.connections.len(),
            sample.status,
            app.snapshot.timestamp.saturating_sub(sample.sampled_at),
            app.snapshot
                .timestamp
                .saturating_sub(sample.owners_sampled_at),
            if !sample.owner_scan_complete || sample.unreadable_processes > 0 {
                " · partial owners"
            } else {
                ""
            },
            if sample.truncated {
                " · truncated"
            } else {
                ""
            }
        )
    };
    if parts[0].height > 0 {
        frame.render_widget(
            Paragraph::new(info).block(block(" NETWORK ", p, p.mint)),
            parts[0],
        );
    }
    let interface_area = if area.width >= 130 && interface_height >= 7 {
        let top = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(parts[1]);
        traffic_chart(frame, top[1], app, p);
        top[0]
    } else {
        parts[1]
    };
    super::panes::draw(frame, interface_area, 1, app, p, interfaces);
    super::panes::draw(frame, parts[2], 0, app, p, connections);
}
fn connections(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let title = if let Some(scope) = &app.object_scope {
        format!(" CONNECTIONS · {} ", scope.title)
    } else {
        " CONNECTIONS ".into()
    };
    let b = super::controls::selectable_block(title, area, app, p, p.mint);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let table_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(2),
    );
    let sort = C::at(app.connection_sort);
    let tag_width = super::tags::column_width(inner.width);
    let body_width = inner.width.saturating_sub(tag_width + 1);
    let columns = connection_columns(body_width, sort);
    let mut widths: Vec<_> = columns
        .iter()
        .map(|c| connection_width(*c, body_width))
        .collect();
    widths.push(Constraint::Length(tag_width));
    header_regions(app, table_area, &widths, columns.iter().map(|c| c.index()));
    let column_rects = super::tags::columns(table_area, &widths, 1);
    let tag_rect = *column_rects.last().unwrap();
    let rows = app.filtered_connections();
    let empty = rows.is_empty();
    let plans: Vec<_> = rows
        .iter()
        .map(|row| {
            super::tags::plan(
                &row.tags,
                String::new(),
                tag_rect.width,
                p,
                app.config.no_color,
            )
        })
        .collect();
    let data: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            Row::new(
                columns
                    .iter()
                    .enumerate()
                    .map(|(j, column)| {
                        if *column == C::State {
                            visuals::state_cell(&row.state, p)
                        } else {
                            Cell::from(super::tags::elide(&column.cell(row), column_rects[j].width))
                                .style(Style::new().fg(match column {
                                    C::Download | C::Received => p.mint,
                                    C::Upload | C::Sent => p.blue,
                                    C::Retransmits if row.retransmits.is_some_and(|v| v > 0) => {
                                        p.amber
                                    }
                                    _ => p.text,
                                }))
                        }
                    })
                    .chain(std::iter::once(plans[i].cell())),
            )
        })
        .collect();
    let headers = columns
        .iter()
        .map(|c| {
            heading(
                c.header(),
                *c == sort,
                sort.descending(app.connection_reverse),
                p,
            )
        })
        .chain(std::iter::once(
            Cell::from("LABELS").style(Style::new().fg(p.muted)),
        ));
    let mut state = TableState::default()
        .with_offset(app.table_offset)
        .with_selected(if empty || !app.panel_focused {
            None
        } else {
            Some(app.selected)
        });
    frame.render_stateful_widget(
        Table::new(data, widths)
            .header(Row::new(headers))
            .column_spacing(1)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(Style::new().bg(p.selection))
            .highlight_symbol("▎"),
        table_area,
        &mut state,
    );
    app.table_offset = state.offset();
    super::tags::render_column(frame, &plans, tag_rect, app.table_rect, state.offset());
    if inner.height >= 3 {
        let note = if let Some(note) = app.snapshot.connections.notes.first() {
            note.clone()
        } else {
            format!(
                " TCP counters · unknown values shown as — · {} unreadable processes",
                app.snapshot.connections.unreadable_processes
            )
        };
        frame.render_widget(
            Paragraph::new(note).style(Style::new().fg(p.muted)),
            Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
        );
    }
    if empty && app.table_rect.height > 1 {
        frame.render_widget(Paragraph::new("No matching sockets. [Del] clears the filter; permission/namespace limits are shown above.").wrap(Wrap{trim:false}).style(Style::new().fg(p.muted)),app.table_rect);
    }
}
fn traffic_chart(frame: &mut Frame, area: Rect, app: &App, p: Palette) {
    super::overview::network_card(frame, area, app, p);
}
fn interfaces(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let count = app.filtered_networks().len();
    let title = format!(" INTERFACES · {count} ");
    let b = super::controls::selectable_block(title, area, app, p, p.mint);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let sort = I::at(app.interface_sort);
    let mut columns = if inner.width >= 120 {
        vec![
            I::Name,
            I::State,
            I::Download,
            I::Upload,
            I::Received,
            I::Sent,
            I::Errors,
            I::Address,
        ]
    } else if inner.width >= 90 {
        vec![
            I::Name,
            I::State,
            I::Download,
            I::Upload,
            I::Received,
            I::Sent,
            I::Errors,
        ]
    } else {
        vec![I::Name, I::Download, I::Upload, I::Errors]
    };
    if !columns.contains(&sort) {
        if inner.width < 120 {
            columns.pop();
        }
        columns.push(sort);
    }
    let widths: Vec<_> = columns
        .iter()
        .map(|c| match c {
            I::Name => Constraint::Length(16),
            I::State => Constraint::Length(9),
            I::Address => Constraint::Min(15),
            _ => Constraint::Min(12),
        })
        .collect();
    header_regions(app, inner, &widths, columns.iter().map(|c| c.index()));
    let headers = columns.iter().map(|c| {
        heading(
            c.header(),
            *c == sort,
            sort.descending(app.interface_reverse),
            p,
        )
    });
    let networks = app.filtered_networks();
    let data = networks.iter().map(|n| {
        Row::new(columns.iter().map(|c| {
            if *c == I::State {
                visuals::state_cell(&n.state, p)
            } else {
                Cell::from(c.cell(n)).style(Style::new().fg(match c {
                    I::Download | I::Received => p.mint,
                    I::Upload | I::Sent => p.blue,
                    I::Errors if n.errors > 0 => p.red,
                    I::Drops if n.drops.is_some_and(|v| v > 0) => p.amber,
                    _ => p.text,
                }))
            }
        }))
    });
    let mut state = TableState::default()
        .with_offset(app.table_offset)
        .with_selected(if networks.is_empty() || !app.panel_focused {
            None
        } else {
            Some(app.selected)
        });
    frame.render_stateful_widget(
        Table::new(data, widths)
            .header(Row::new(headers))
            .column_spacing(1)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(Style::new().bg(p.selection))
            .highlight_symbol("▎"),
        inner,
        &mut state,
    );
    app.table_offset = state.offset();
}
pub fn detail(app: &App) -> Option<(String, String)> {
    let c = *app.filtered_connections().get(app.selected)?;
    let mut body = format!(
        "{} → {}\n{} · {}\n\n{}\n\nProcess names: {}\nPIDs: {:?}\nReceived: {} · Sent: {}\nReceive queue: {} · Send queue: {}\nTCP RTT: {} · Retransmits: {}\n\nRates are deltas of kernel TCP payload counters between samples, not wire speed. Queues are waiting bytes except listening sockets, where TCP queue columns are not traffic.\nOwners are sampled separately; exited/short-lived/restricted processes can be unknown.\n\nClose this detail and use Tools to test the target, open its process or capture packets.\nDomain names, encrypted contents, and proxy routing rules are not inferred from IP addresses.\n",
        c.local,
        c.remote,
        c.protocol,
        c.state,
        crate::connections::state_help(&c.state),
        c.process_names.join(", "),
        c.pids,
        optional_bytes(c.received),
        optional_bytes(c.sent),
        optional_bytes(c.receive_queue),
        optional_bytes(c.send_queue),
        optional(c.rtt_ms.map(|v| format!("{v:.2} ms")), ""),
        optional(c.retransmits, "")
    );
    for pid in &c.pids {
        if let Some(p) = app.snapshot.processes.iter().find(|p| p.pid == *pid) {
            body.push_str(&format!(
                "\nPID {pid} · {} · {}\n{}\n",
                p.name, p.user, p.command
            ));
        }
    }
    body.push_str(&format!(
        "\nRecognized labels\n{}\nClose detail, then Y opens label-specific actions.\n",
        crate::tags::explain(&c.tags)
    ));
    Some(("CONNECTION DETAILS".into(), body))
}
