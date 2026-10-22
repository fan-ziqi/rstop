use super::{Palette, block};
use crate::app::App;
use ratatui::{prelude::*, widgets::*};
pub(super) fn menu(
    frame: &mut Frame,
    title: &str,
    items: &[(String, crossterm::event::KeyCode)],
    selected: usize,
    p: Palette,
) -> (Rect, usize, Rect) {
    let area = popup(frame.area(), 78, (items.len() + 4).min(28) as u16);
    frame.render_widget(Clear, area);
    let title = super::tags::elide(title, area.width.saturating_sub(4));
    let b = block(format!(" {title} "), p, p.mint)
        .title_bottom(" [↑↓] Select · [Enter] Apply · [Esc] Back ");
    let inner = b.inner(area);
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items.iter().enumerate().map(|(index, (label, key))| {
            let hint = if *key == crossterm::event::KeyCode::Null {
                if index == selected {
                    "[Enter]".into()
                } else {
                    String::new()
                }
            } else {
                format!("[{}]", crate::app::commands::key_name(*key))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {hint:<9}"), Style::new().fg(p.mint).bold()),
                Span::styled(label.clone(), Style::new().fg(p.text)),
            ]))
        }))
        .block(b)
        .highlight_style(Style::new().bg(p.selection).fg(p.mint).bold())
        .highlight_symbol("▸"),
        area,
        &mut state,
    );
    (inner, state.offset(), area)
}

pub(super) fn surface(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    p: Palette,
    accent: Color,
) -> Rect {
    let shadow =
        Rect::new(area.x + 2, area.y + 1, area.width, area.height).intersection(frame.area());
    let shadow_color = match p.bg {
        Color::Rgb(r, g, b) => Color::Rgb(r / 3, g / 3, b / 3),
        color => color,
    };
    frame.render_widget(Block::new().style(Style::new().bg(shadow_color)), shadow);
    frame.render_widget(Clear, area);
    let title = super::tags::elide(title.trim(), area.width.saturating_sub(6));
    let border = block(format!(" {title} "), p, accent).padding(Padding::new(2, 2, 1, 1));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    inner
}

pub(super) fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

pub(super) fn settings(frame: &mut Frame, app: &mut App, p: Palette) {
    let area = popup(frame.area(), 74, 23);
    app.settings_rect = area;
    app.popup_rect = area;
    frame.render_widget(Clear, area);
    let theme_index = crate::config::THEMES
        .iter()
        .position(|name| *name == app.config.theme)
        .unwrap_or(0);
    let options = [
        (
            "Theme",
            format!(
                "{}  ({}/{})",
                app.config.theme,
                theme_index + 1,
                crate::config::THEMES.len()
            ),
        ),
        (
            "Sampling interval",
            format!("{} ms", app.config.interval_ms),
        ),
        (
            "History retention",
            format!("{} samples", app.config.history),
        ),
        ("Show loopback", app.config.show_loopback.to_string()),
        ("CPU warning", format!("{:.0}%", app.config.cpu_warning)),
        (
            "Memory warning",
            format!("{:.0}%", app.config.memory_warning),
        ),
        (
            "GPU temperature warning",
            format!("{}°C", app.config.gpu_temperature_warning),
        ),
    ];
    let mut options = options.to_vec();
    options.extend([
        ("Layout preset", app.config.layout.clone()),
        ("Show threads", app.config.show_threads.to_string()),
        (
            "Show kernel threads",
            app.config.show_kernel_threads.to_string(),
        ),
        ("ASCII glyphs", app.config.ascii.to_string()),
        ("No color", app.config.no_color.to_string()),
        ("Alert hold", format!("{} s", app.config.alert_hold_seconds)),
        ("Log tail lines", app.config.log_lines.to_string()),
    ]);
    options.extend(app.config.display.settings());
    let mut lines = vec![
        Line::styled(
            " [↑↓] Choose · [←→] Adjust · [w] Save · [Esc] Back",
            Style::new().fg(p.muted),
        ),
        Line::raw(""),
    ];
    let visible = area.height.saturating_sub(6).max(1) as usize;
    app.setting_offset = app.setting_index.saturating_sub(visible.saturating_sub(1));
    for (i, (label, value)) in options
        .iter()
        .enumerate()
        .skip(app.setting_offset)
        .take(visible)
    {
        let (display, color) = match value.as_str() {
            "true" => ("(●) ON".to_string(), p.mint),
            "false" => ("(○) OFF".to_string(), p.muted),
            _ => (value.clone(), p.blue),
        };
        let mut line = Line::from(vec![
            Span::styled(
                format!(" {} ", if i == app.setting_index { "▸" } else { " " }),
                Style::new().fg(p.mint),
            ),
            Span::styled(format!("{label:<28} "), Style::new().fg(p.text)),
            Span::styled(display, Style::new().fg(color).bold()),
        ]);
        if i == app.setting_index {
            line = line.style(Style::new().bg(p.selection));
        }
        lines.push(line);
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!(" {}", app.status),
        Style::new().fg(p.muted),
    ));
    frame.render_widget(
        Paragraph::new(lines).block(block(" PREFERENCES ", p, p.mint)),
        area,
    );
}

pub(super) fn detail(
    frame: &mut Frame,
    title: &str,
    body: &str,
    scroll: usize,
    p: Palette,
) -> Rect {
    detail_window(frame, title, body, scroll, None, p)
}

pub(super) fn notice(
    frame: &mut Frame,
    title: &str,
    body: &str,
    scroll: usize,
    hint: &'static str,
    p: Palette,
) -> Rect {
    render_detail(frame, title, body, (scroll, None), hint, p)
}

pub(super) fn log_detail(
    frame: &mut Frame,
    title: &str,
    body: &str,
    scroll: usize,
    column: u16,
    p: Palette,
) -> Rect {
    detail_window(frame, title, body, scroll, Some(column), p)
}

fn detail_window(
    frame: &mut Frame,
    title: &str,
    body: &str,
    scroll: usize,
    column: Option<u16>,
    p: Palette,
) -> Rect {
    render_detail(
        frame,
        title,
        body,
        (scroll, column),
        " [Esc] Back · [↑↓] Scroll · [/] Search ",
        p,
    )
}

fn render_detail(
    frame: &mut Frame,
    title: &str,
    body: &str,
    (scroll, column): (usize, Option<u16>),
    hint: &'static str,
    p: Palette,
) -> Rect {
    let area = popup(frame.area(), 110, 32);
    frame.render_widget(Clear, area);
    let clean = body
        .lines()
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect::<String>();
    let mut after_gap = true;
    let lines: Vec<Line<'_>> = clean
        .lines()
        .map(|line| {
            let section = column.is_none()
                && after_gap
                && !line.is_empty()
                && line.len() < 65
                && !line.starts_with(char::is_whitespace)
                && !line.contains("  ")
                && !line.contains([':', '{', '}', '=']);
            after_gap = line.trim().is_empty();
            if column.is_some() {
                return Line::raw(line);
            }
            if section {
                return Line::styled(line, Style::new().fg(p.blue).bold());
            }
            if let Some(index) = line.find("  ") {
                let (label, value) = line.split_at(index);
                if !label.trim().is_empty() && label.len() <= 32 {
                    return Line::from(vec![
                        Span::styled(label, Style::new().fg(p.muted)),
                        Span::styled(value, Style::new().fg(p.text)),
                    ]);
                }
            }
            Line::raw(line)
        })
        .collect();
    let widget =
        Paragraph::new(lines).block(block(format!(" {title} "), p, p.blue).title_bottom(hint));
    let widget = if let Some(column) = column {
        widget.scroll((scroll.min(u16::MAX as usize) as u16, column))
    } else {
        widget
            .wrap(Wrap { trim: false })
            .scroll((scroll.min(u16::MAX as usize) as u16, 0))
    };
    frame.render_widget(widget, area);
    area
}

pub(super) fn help(frame: &mut Frame, app: &App, p: Palette) -> Rect {
    let body = format!(
        "rstop {} · one terminal, your whole system

COMMON CONTROLS
Keys always appear in [brackets]. Click a control or press its key.
  [←/→]       Previous / next resource page (wraps around)
  [1]–[8]      Resource pages; [Tab] / [Shift+Tab] also switch pages
  [F3] Filter  Search the current table
  [F4] Sort    Choose a field or reverse the current order
  [F5] View    Panel focus, chart window, zoom and sampling
  [-/+]       Decrease / increase sampling interval; also clickable at the top
  [Enter]      Open / close the selected row's detail pane; [d] full details
  [F6] Tools   Actions for the current resource or selected object
  [F7] Logs    Process output, service journal or container logs
  [Esc]        Back: close pane/popup, unselect, clear filter, then menu
  [F1] Help    [F2] Settings    [Ctrl+C] Quit
  Press [Esc] twice within 500 ms to quit from any screen.

SELECT AND SEARCH
  [↑↓] select; [PgUp/PgDn] scroll; [Home/End] jump.
  Click rows to select; double-click for details; right-click for tools.
  Click outside a popup to close it; parent windows stay open.
  Click a column heading to sort; click it again to reverse.
  [F3] edits in place: [Enter] applies, [↓] opens results,
  [Esc] restores the previous query, [Del] clears an applied filter.
  Text editors support arrows, Home/End and Ctrl+A/E/U/K/W.
  Filters: name:python  user:alice cpu:>10  !name:rust
  Regex: name:/python|torch/     OR: pid:42 | mem:>1024
  Select a row, then [Enter] opens its pane. Selection alone stays in the table.
  Scroll details with the wheel or [Alt+↑↓/PgUp/PgDn/Home/End].
  Click detail labels or use [F6] for related actions.

NETWORK
  Interfaces and Connections share one page. Click or scroll a table
  to focus it, or choose focus in [F5]. Each has its own filter and scroll.
  [F4] has 18 connection fields or 13 interface fields plus Reverse.
  Process and PID are separate columns and sort independently.
  Filter examples: port:443  process:python !proto:udp  state:LISTEN
  [F6] Tools: test a website/service, capture packets, open a PCAP,
  routes and DNS, diagnose problems, or open the owning process.
  Capture guides you through interface, filter and output, then asks
  for confirmation. Missing counters are shown as —.

OTHER RESOURCE TOOLS
  Processes: details, output/logs, environment, open files, columns;
  Advanced process tools contains signals, priority, affinity and tracing.
  Storage: large folders, inodes, deleted-open files and kernel messages.
  Services / containers: logs, details and management with confirmation.
  GPU: cards, charts and processes share one page.
  [I] Device info or double-click a GPU card opens its searchable popup.
  [F5] also selects another GPU.
  Overview GPU card: click Prev or Next, or choose a GPU action in [F5].
  Press '[' for the previous GPU, ']' for the next GPU; selection wraps around.
  Diagnosis includes trends, related alerts and investigation steps.

COMPATIBILITY SHORTCUTS
These remain available; menus show the key beside each action.
  [f] or [/] Filter; [s] Sort; [r] Reverse; [D] Other network table / GPU info
  [N/C/F] Network test / capture / read PCAP; [p] Owning process
  [R/O/J] Diagnose / additional details / Linux kernel and OOM
  [F7] or [l] Output/logs; [e/o] Environment / open files
  [C] Process columns; [Shift+←/→] Column window
  [t] Process tree; [Ctrl+←/→] Collapse / expand or scroll commands
  [P/M] CPU / RAM sort
  [g/u/H/h] GPU processes / own processes / threads / kernel threads
  [v/V/U] Mark / mark descendants / clear marks; [Y] Label actions
  [n/a/L/x/B] Nice / affinity / locks / trace / backtrace
  [F9/K] Signal menu / SIGKILL; [y] confirms fixed targets
  [W/z/Z] Chart window / zoom in / out; [+/-] Sampling speed
  [S/T] GPU processes; [I] Device table; [m] Advanced process / workload tools
  [,/.] Replay backward / forward ten samples
  [?/Esc] Main menu; [:] All commands; [c/i/w] Theme / loopback / save
  [q] or [Ctrl+C] Quit (q is text while editing); [F10] also quits if received

LOG DETAILS
  [f] Follow/pause; [p] Choose file; [/] Search; [←→] Pan.
  Scrolling pauses following. [Esc] returns to the resource.

Missing/unsupported values are —. Live actions are disabled in replay.
CPU% per process uses 100% per logical core. GPU% may sum across devices.
Config: {}",
        env!("CARGO_PKG_VERSION"),
        crate::config::Config::path().display()
    );
    detail(
        frame,
        &format!(" KEYBOARD  ·  {} ", app.config.theme),
        &body,
        app.detail_scroll,
        p,
    )
}

pub(super) fn parse_color(value: &str) -> Option<Color> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    Some(Color::Rgb(
        (number >> 16) as u8,
        (number >> 8) as u8,
        number as u8,
    ))
}
