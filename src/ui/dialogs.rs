use super::{Palette, block};
use crate::app::App;
use ratatui::{prelude::*, widgets::*};
pub(super) fn menu(
    frame: &mut Frame,
    title: &str,
    items: &[(String, crossterm::event::KeyCode)],
    selected: usize,
    offset: usize,
    kind: crate::app::MenuKind,
    p: Palette,
) -> (Rect, usize, Rect, Rect) {
    let width = items
        .iter()
        .map(|(label, key)| {
            Line::raw(label).width() + crate::app::commands::key_name(*key).len() + 11
        })
        .max()
        .unwrap_or(44)
        .clamp(48, 78) as u16;
    let area = popup(frame.area(), width, (items.len() + 6).clamp(8, 28) as u16);
    let inner = surface(frame, area, title, p, p.mint);
    let mut list_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(2),
    );
    let selected = selected.min(items.len().saturating_sub(1));
    let rows = list_area.height.max(1) as usize;
    let mut offset = offset.min(items.len().saturating_sub(rows));
    if selected < offset {
        offset = selected;
    }
    if selected >= offset + rows {
        offset = selected + 1 - rows;
    }
    let scrolling = items.len() > rows;
    if scrolling {
        list_area.width = list_area.width.saturating_sub(1);
    }
    let mut state = ListState::default()
        .with_selected(if items.is_empty() {
            None
        } else {
            Some(selected)
        })
        .with_offset(offset);
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
            let label_width = list_area.width.saturating_sub(hint.len() as u16 + 4);
            let label = super::tags::elide(label, label_width);
            let gap = list_area
                .width
                .saturating_sub(Line::raw(&label).width() as u16 + hint.len() as u16 + 2);
            ListItem::new(Line::from(vec![
                Span::styled(label, Style::new().fg(p.text)),
                Span::raw(" ".repeat(gap as usize)),
                Span::styled(
                    hint,
                    Style::new().fg(if index == selected { p.mint } else { p.muted }),
                ),
            ]))
        }))
        .highlight_style(Style::new().bg(p.selection).fg(p.text))
        .highlight_symbol("▸ "),
        list_area,
        &mut state,
    );
    let back = Rect::new(inner.right().saturating_sub(11), inner.bottom() - 1, 11, 1);
    let instructions = if kind == crate::app::MenuKind::Columns {
        "[Space/Enter] Toggle"
    } else {
        "[↑↓/Tab] Move [Enter] Open"
    };
    frame.render_widget(
        Paragraph::new(super::tags::elide(
            instructions,
            inner.width.saturating_sub(12),
        ))
        .style(Style::new().fg(p.muted)),
        Rect::new(inner.x, back.y, inner.width.saturating_sub(12), 1),
    );
    super::controls::nav_button(
        frame,
        back,
        crossterm::event::KeyCode::Esc,
        if kind == crate::app::MenuKind::Columns {
            "Done"
        } else {
            "Back"
        },
        false,
        p,
        p.panel,
    );
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No items to display").style(Style::new().fg(p.muted)),
            list_area,
        );
    }
    if scrolling {
        let mut scroll = super::scrollbar::state(items.len(), rows, offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint))
                .track_style(Style::new().fg(p.border)),
            Rect::new(list_area.right(), list_area.y, 1, list_area.height),
            &mut scroll,
        );
        frame.render_widget(
            Paragraph::new(format!(" {}/{} ", selected + 1, items.len()))
                .style(Style::new().fg(p.muted))
                .alignment(Alignment::Right),
            Rect::new(area.x + 2, area.bottom() - 1, area.width - 4, 1),
        );
    }
    (list_area, state.offset(), area, back)
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
    let area = popup(frame.area(), 78, 26);
    app.settings_rect = area;
    app.popup_rect = area;
    let inner = surface(frame, area, "Settings", p, p.mint);
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
    app.popup_actions.clear();
    app.dialog.settings_rows.clear();
    app.dialog.settings_adjust.clear();
    let visible = area.height.saturating_sub(7).max(1) as usize;
    app.dialog.rows = visible;
    app.dialog.lines = options.len();
    let mut offset = app
        .setting_offset
        .min(options.len().saturating_sub(visible));
    if app.setting_index < offset {
        offset = app.setting_index;
    }
    if app.setting_index >= offset + visible {
        offset = app.setting_index + 1 - visible;
    }
    app.setting_offset = offset;
    frame.render_widget(
        Paragraph::new(format!(
            "[↑↓/Tab] Choose · [←→/Enter] Adjust   {}/{}",
            app.setting_index + 1,
            options.len()
        ))
        .style(Style::new().fg(p.muted)),
        Rect::new(inner.x, area.y + 1, inner.width, 1),
    );
    let value_width = 17.min(inner.width / 2);
    for (index, (label, value)) in options.iter().enumerate().skip(offset).take(visible) {
        let row = Rect::new(
            inner.x,
            area.y + 3 + (index - offset) as u16,
            inner.width.saturating_sub(1),
            1,
        );
        let active = app.setting_index == index;
        if active {
            frame.render_widget(Block::new().style(Style::new().bg(p.selection)), row);
        }
        let right = Rect::new(row.right() - 3, row.y, 3, 1);
        let left = Rect::new(right.x - value_width - 4, row.y, 3, 1);
        let label_rect = Rect::new(row.x + 2, row.y, left.x.saturating_sub(row.x + 3), 1);
        frame.render_widget(
            Paragraph::new(if active { "▸" } else { " " }).style(Style::new().fg(p.mint)),
            Rect::new(row.x, row.y, 1, 1),
        );
        frame.render_widget(
            Paragraph::new(super::tags::elide(label, label_rect.width))
                .style(Style::new().fg(p.text)),
            label_rect,
        );
        let (display, color) = match value.as_str() {
            "true" => ("On", p.mint),
            "false" => ("Off", p.muted),
            _ => (value.as_str(), p.blue),
        };
        frame.render_widget(
            Paragraph::new("[←]").style(Style::new().fg(if active { p.mint } else { p.border })),
            left,
        );
        frame.render_widget(
            Paragraph::new(super::tags::elide(display, value_width))
                .alignment(Alignment::Center)
                .style(Style::new().fg(color)),
            Rect::new(left.right() + 1, row.y, value_width, 1),
        );
        frame.render_widget(
            Paragraph::new("[→]").style(Style::new().fg(if active { p.mint } else { p.border })),
            right,
        );
        app.dialog.settings_rows.push((row, index));
        app.dialog.settings_adjust.extend([
            (left, index, crossterm::event::KeyCode::Left),
            (right, index, crossterm::event::KeyCode::Right),
        ]);
    }
    if options.len() > visible {
        let mut state = super::scrollbar::state(options.len(), visible, offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint))
                .track_style(Style::new().fg(p.border)),
            Rect::new(inner.right() - 1, area.y + 3, 1, visible as u16),
            &mut state,
        );
    }
    let message = if app.dialog.settings_message.is_empty() {
        options
            .get(app.setting_index)
            .map(|(label, _)| *label)
            .unwrap_or("")
    } else {
        &app.dialog.settings_message
    };
    frame.render_widget(
        Paragraph::new(super::tags::elide(message, inner.width)).style(Style::new().fg(
            if message.starts_with("Could not") {
                p.red
            } else {
                p.muted
            },
        )),
        Rect::new(inner.x, inner.bottom() - 2, inner.width, 1),
    );
    let y = inner.bottom() - 1;
    for (rect, key, label) in [
        (
            Rect::new(inner.x, y, 26, 1),
            crossterm::event::KeyCode::Char('w'),
            "Save preferences",
        ),
        (
            Rect::new(inner.right() - 14, y, 14, 1),
            crossterm::event::KeyCode::Esc,
            "Back",
        ),
    ] {
        super::controls::nav_button(frame, rect, key, label, false, p, p.panel);
        app.popup_actions.push((rect, key));
    }
}

pub(super) fn detail(
    frame: &mut Frame,
    app: &mut App,
    title: &str,
    body: &str,
    p: Palette,
) -> Rect {
    render_detail(frame, app, title, body, false, p)
}

pub(super) fn log_detail(
    frame: &mut Frame,
    app: &mut App,
    title: &str,
    body: &str,
    p: Palette,
) -> Rect {
    render_detail(frame, app, title, body, true, p)
}

fn render_detail(
    frame: &mut Frame,
    app: &mut App,
    title: &str,
    body: &str,
    log: bool,
    p: Palette,
) -> Rect {
    use crossterm::event::KeyCode;
    app.popup_actions.clear();
    let clean = body
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect::<String>();
    let mut after_gap = true;
    let mut lines: Vec<Line<'_>> = clean
        .lines()
        .map(|line| {
            let section = !log
                && after_gap
                && !line.is_empty()
                && line.len() < 65
                && !line.starts_with(char::is_whitespace)
                && !line.contains("  ")
                && !line.contains([':', '{', '}', '=']);
            after_gap = line.trim().is_empty();
            if section {
                Line::styled(line, Style::new().fg(p.blue).bold())
            } else {
                Line::raw(line)
            }
        })
        .collect();
    let empty = lines.is_empty();
    if empty {
        lines.push(Line::styled(
            if app.detail_filter.is_empty() {
                "No content to display"
            } else {
                "No matching lines"
            },
            Style::new().fg(p.muted),
        ));
    }
    let width = (lines.iter().map(Line::width).max().unwrap_or(0) + 7).clamp(52, 110) as u16;
    let content_width = width
        .min(frame.area().width.saturating_sub(4))
        .saturating_sub(7)
        .max(1);
    let measured = Paragraph::new(lines.clone()).wrap(Wrap { trim: false });
    let rows = if log {
        lines.len()
    } else {
        measured.line_count(content_width)
    };
    let area = popup(frame.area(), width, (rows + 6).clamp(9, 32) as u16);
    let inner = surface(frame, area, title, p, p.blue);
    let content = Rect::new(
        inner.x,
        inner.y,
        inner.width.saturating_sub(1),
        inner.height.saturating_sub(2),
    );
    app.detail_rows = content.height as usize;
    app.detail_width = content.width;
    app.dialog.lines = rows;
    app.dialog.rows = content.height as usize;
    app.dialog.content = content;
    let maximum = rows.saturating_sub(content.height as usize);
    if log && app.follow_logs {
        app.detail_scroll = maximum;
    } else {
        app.detail_scroll = app.detail_scroll.min(maximum);
    }
    let longest = lines.iter().map(Line::width).max().unwrap_or(0);
    app.detail_column = app.detail_column.min(
        longest
            .saturating_sub(content.width as usize)
            .min(u16::MAX as usize) as u16,
    );
    let widget = Paragraph::new(lines);
    let widget = if log {
        widget.scroll((
            app.detail_scroll.min(u16::MAX as usize) as u16,
            app.detail_column,
        ))
    } else {
        widget
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll.min(u16::MAX as usize) as u16, 0))
    };
    frame.render_widget(widget, content);
    app.text_regions.clear();
    app.text_regions
        .push(crate::app::text_selection::TextRegion::capture(
            format!(
                "report:{title}:{}:{}",
                app.detail_generation, app.detail_filter
            ),
            (app.detail_scroll, if log { app.detail_column } else { 0 }),
            content,
            frame.buffer_mut(),
        ));
    if rows > content.height as usize {
        let mut state = super::scrollbar::state(rows, content.height as usize, app.detail_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::new().fg(p.mint))
                .track_style(Style::new().fg(p.border)),
            Rect::new(content.right(), content.y, 1, content.height),
            &mut state,
        );
    }
    let status = Rect::new(inner.x, inner.bottom() - 2, inner.width, 1);
    if app.detail_search || !app.detail_filter.is_empty() {
        let prefix = "Find: ";
        let available = status.width.saturating_sub(prefix.len() as u16 + 1);
        let cursor =
            Line::raw(&app.detail_filter[..app.editor.position(&app.detail_filter)]).width() as u16;
        let scroll = cursor.saturating_sub(available.saturating_sub(1));
        frame.render_widget(
            Paragraph::new(prefix).style(Style::new().fg(p.blue)),
            Rect::new(status.x, status.y, prefix.len() as u16, 1),
        );
        let field = Rect::new(status.x + prefix.len() as u16, status.y, available, 1);
        frame.render_widget(
            Paragraph::new(app.detail_filter.as_str())
                .scroll((0, scroll))
                .style(Style::new().fg(p.text).bg(p.selection)),
            field,
        );
        app.dialog.input = field;
        if app.detail_search {
            frame.set_cursor_position((
                field.x
                    + cursor
                        .saturating_sub(scroll)
                        .min(available.saturating_sub(1)),
                field.y,
            ));
        }
    } else {
        let status_text = if empty {
            "0 lines".into()
        } else {
            format!(
                "{}–{} / {} lines{}",
                app.detail_scroll + 1,
                (app.detail_scroll + content.height as usize).min(rows),
                rows,
                if log {
                    if app.follow_logs {
                        " · Following"
                    } else {
                        " · Paused"
                    }
                } else {
                    ""
                }
            )
        };
        frame.render_widget(
            Paragraph::new(status_text).style(Style::new().fg(p.muted)),
            status,
        );
    }
    let mut x = inner.x;
    let mut buttons = vec![(KeyCode::Esc, "Back"), (KeyCode::Char('/'), "Find")];
    if log {
        buttons.extend([
            (
                KeyCode::Char('f'),
                if app.follow_logs { "Pause" } else { "Follow" },
            ),
            (KeyCode::Char('p'), "File"),
        ]);
    }
    for (key, label) in buttons {
        let width = (crate::app::commands::key_name(key).len() + label.len() + 5) as u16;
        let rect = Rect::new(x, inner.bottom() - 1, width, 1);
        if rect.right() > inner.right() {
            break;
        }
        super::controls::nav_button(frame, rect, key, label, false, p, p.panel);
        app.popup_actions.push((rect, key));
        x += width;
    }
    area
}

pub(super) fn help(frame: &mut Frame, app: &mut App, p: Palette) -> Rect {
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
  Drag across pane/report/log text to select; Ctrl+C or Ctrl+Shift+C copies.
  Esc clears text selection. Ctrl+C without a text selection quits.
  Filters: name:python  user:alice cpu:>10  !name:rust
  Regex: name:/python|torch/     OR: pid:42 | mem:>1024
  Select a row, then [Enter] opens its pane. Selection alone stays in the table.
  Scroll details with the wheel or [Alt+↑↓/PgUp/PgDn/Home/End].
  Click detail labels or use [F6] for related actions.

POPUPS
  [↑↓/Tab/Shift+Tab] Select; [Home/End] First/last; [PgUp/PgDn] Page.
  Settings: [←→/Enter] adjust or click a row's arrows; [w] saves.
  Input errors keep your value and show the reason in the form.
  Confirmations: [y/Enter] Confirm; [n/Esc] Cancel.
  Long previews scroll before confirmation. Reports/help: [/] Find.
  [Esc] or Back returns to the parent; clicks do not pass through.

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
  [T] Process tree; [Ctrl+←/→] Collapse / expand or scroll commands
  [P/M] CPU / RAM sort
  [g/u/H/h] GPU processes / own processes / threads / kernel threads
  [Space/v] Mark; [V/U] Mark descendants / clear marks; [Y] Label actions
  [n/N] Nice; [a/L/x/B] Affinity / locks / trace / backtrace
  [k/K] Kill (SIGKILL); [t] Terminate (SIGTERM); [s/F9] Signal menu
  Signal dialog: [i/t/k/s/c] INT / TERM / KILL / STOP / CONT
  All supported signals are listed. [↑↓←→] Select; [Home/End] First/last
  [PgUp/PgDn] Page signals; [Alt+↑↓] Scroll targets; [Tab/Shift+Tab] Next/previous signal
  [Enter/y] Confirms fixed targets; [Esc/n] Cancels
  [Alt+j/k] Select next / previous process; [↑↓] also select
  [W/z/Z] Chart window / zoom in / out; [+/-] Sampling speed
  [S] GPU processes; [I] Device table; [m] Advanced process / workload tools
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
    let body = if app.detail_filter.is_empty() {
        body
    } else {
        body.lines()
            .filter(|line| {
                line.to_lowercase()
                    .contains(&app.detail_filter.to_lowercase())
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    detail(frame, app, "Keyboard shortcuts", &body, p)
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
