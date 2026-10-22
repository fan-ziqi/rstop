mod controls;
mod details;
mod dialogs;
mod gpu;
mod main_menu;
pub mod network;
mod overview;
mod panes;
mod processes;
mod resources;
mod selection;
mod tags;
mod updates;
mod visuals;
mod wordmark;
mod workloads;
use details::{event_view, explorer, history_view};
use dialogs::{detail, help, log_detail, menu, parse_color, settings};
use gpu::gpu_view;
use overview::overview;
use processes::{process_summary, process_table};
use resources::{network_view, storage_view, system_view};
use workloads::{containers, services};
mod charts;
use crate::{
    app::{App, View},
    model::*,
};
use ratatui::{prelude::*, widgets::*};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub border: Color,
    pub muted: Color,
    pub text: Color,
    pub mint: Color,
    pub blue: Color,
    pub purple: Color,
    pub amber: Color,
    pub red: Color,
    pub selection: Color,
}
pub fn palette(name: &str) -> Palette {
    match name {
        "black" => Palette {
            bg: Color::Rgb(0, 0, 0),
            panel: Color::Rgb(10, 10, 10),
            border: Color::Rgb(55, 55, 60),
            muted: Color::Rgb(160, 166, 176),
            text: Color::Rgb(235, 238, 243),
            mint: Color::Rgb(105, 205, 171),
            blue: Color::Rgb(124, 179, 239),
            purple: Color::Rgb(189, 163, 228),
            amber: Color::Rgb(226, 186, 111),
            red: Color::Rgb(239, 146, 154),
            selection: Color::Rgb(25, 41, 48),
        },
        "white" => Palette {
            bg: Color::Rgb(245, 247, 250),
            panel: Color::Rgb(255, 255, 255),
            border: Color::Rgb(173, 184, 198),
            muted: Color::Rgb(82, 96, 114),
            text: Color::Rgb(28, 39, 55),
            mint: Color::Rgb(0, 109, 88),
            blue: Color::Rgb(32, 87, 162),
            purple: Color::Rgb(112, 64, 159),
            amber: Color::Rgb(132, 84, 14),
            red: Color::Rgb(174, 42, 59),
            selection: Color::Rgb(224, 234, 246),
        },
        "graphite" => Palette {
            bg: Color::Rgb(22, 22, 24),
            panel: Color::Rgb(29, 29, 32),
            border: Color::Rgb(63, 63, 70),
            muted: Color::Rgb(161, 161, 170),
            text: Color::Rgb(235, 235, 239),
            mint: Color::Rgb(163, 205, 184),
            blue: Color::Rgb(151, 180, 214),
            purple: Color::Rgb(191, 174, 215),
            amber: Color::Rgb(213, 189, 143),
            red: Color::Rgb(224, 158, 166),
            selection: Color::Rgb(49, 51, 58),
        },
        "ocean" => Palette {
            bg: Color::Rgb(8, 24, 36),
            panel: Color::Rgb(12, 32, 48),
            border: Color::Rgb(41, 71, 91),
            muted: Color::Rgb(143, 173, 190),
            text: Color::Rgb(219, 235, 245),
            mint: Color::Rgb(103, 197, 187),
            blue: Color::Rgb(116, 183, 230),
            purple: Color::Rgb(175, 166, 220),
            amber: Color::Rgb(218, 189, 137),
            red: Color::Rgb(232, 154, 169),
            selection: Color::Rgb(22, 58, 79),
        },
        "forest" => Palette {
            bg: Color::Rgb(13, 24, 19),
            panel: Color::Rgb(18, 32, 25),
            border: Color::Rgb(48, 73, 58),
            muted: Color::Rgb(151, 177, 157),
            text: Color::Rgb(224, 237, 223),
            mint: Color::Rgb(151, 200, 143),
            blue: Color::Rgb(141, 184, 204),
            purple: Color::Rgb(187, 165, 198),
            amber: Color::Rgb(216, 188, 133),
            red: Color::Rgb(224, 158, 151),
            selection: Color::Rgb(36, 57, 42),
        },
        "sand" => Palette {
            bg: Color::Rgb(244, 237, 224),
            panel: Color::Rgb(253, 248, 238),
            border: Color::Rgb(181, 165, 144),
            muted: Color::Rgb(108, 94, 76),
            text: Color::Rgb(58, 45, 32),
            mint: Color::Rgb(51, 105, 76),
            blue: Color::Rgb(48, 94, 139),
            purple: Color::Rgb(121, 73, 131),
            amber: Color::Rgb(135, 86, 25),
            red: Color::Rgb(171, 53, 48),
            selection: Color::Rgb(232, 219, 196),
        },
        "nord" => Palette {
            bg: Color::Rgb(29, 35, 46),
            panel: Color::Rgb(36, 44, 57),
            border: Color::Rgb(68, 81, 101),
            muted: Color::Rgb(160, 174, 193),
            text: Color::Rgb(229, 233, 240),
            mint: Color::Rgb(145, 181, 178),
            blue: Color::Rgb(143, 181, 195),
            purple: Color::Rgb(190, 168, 186),
            amber: Color::Rgb(204, 184, 143),
            red: Color::Rgb(214, 159, 167),
            selection: Color::Rgb(52, 66, 84),
        },
        "ember" => Palette {
            bg: Color::Rgb(22, 18, 24),
            panel: Color::Rgb(29, 24, 33),
            border: Color::Rgb(73, 55, 70),
            muted: Color::Rgb(176, 155, 172),
            text: Color::Rgb(240, 222, 227),
            mint: Color::Rgb(204, 165, 143),
            blue: Color::Rgb(140, 173, 202),
            purple: Color::Rgb(180, 156, 203),
            amber: Color::Rgb(207, 184, 138),
            red: Color::Rgb(215, 151, 161),
            selection: Color::Rgb(65, 43, 53),
        },
        _ => Palette {
            bg: Color::Rgb(10, 15, 25),
            panel: Color::Rgb(14, 21, 34),
            border: Color::Rgb(42, 58, 80),
            muted: Color::Rgb(143, 160, 182),
            text: Color::Rgb(215, 226, 241),
            mint: Color::Rgb(124, 184, 165),
            blue: Color::Rgb(134, 166, 204),
            purple: Color::Rgb(173, 157, 198),
            amber: Color::Rgb(204, 179, 141),
            red: Color::Rgb(216, 151, 160),
            selection: Color::Rgb(29, 51, 70),
        },
    }
}

fn block<'a>(title: impl Into<Line<'a>>, p: Palette, accent: Color) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.border))
        .title(title.into().style(Style::new().fg(accent).bold()))
        .style(Style::new().bg(p.panel).fg(p.text))
}
fn text(frame: &mut Frame, area: Rect, content: impl Into<Text<'static>>, p: Palette) {
    frame.render_widget(Paragraph::new(content).style(Style::new().fg(p.text)), area);
}
fn meter(label: &str, value: f64, width: usize, color: Color, p: Palette) -> Line<'static> {
    let mut spans = vec![Span::styled(format!("{label} "), Style::new().fg(p.muted))];
    spans.extend(visuals::track(Some(value), 100., width, color, p));
    spans.push(Span::styled(
        format!(" {value:5.1}%"),
        Style::new().fg(color),
    ));
    Line::from(spans)
}
fn health(value: f64, normal: Color, p: Palette) -> Color {
    if value >= 90.0 {
        p.red
    } else if value >= 70.0 {
        p.amber
    } else {
        normal
    }
}
fn optional<T: std::fmt::Display>(value: Option<T>, suffix: &str) -> String {
    value
        .map(|v| format!("{v}{suffix}"))
        .unwrap_or_else(|| "—".into())
}
fn optional_bytes(value: Option<u64>) -> String {
    value.map(|v| bytes(v as f64)).unwrap_or_else(|| "—".into())
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let mut p = palette(&app.config.theme);
    for (key, value) in &app.config.custom_colors {
        if let Some(color) = parse_color(value) {
            match key.as_str() {
                "background" => p.bg = color,
                "panel" => p.panel = color,
                "text" => p.text = color,
                "accent" => p.mint = color,
                "border" => p.border = color,
                _ => {}
            }
        }
    }
    let area = frame.area();
    app.popup_rect = Rect::default();
    app.update_buttons.clear();
    app.tab_ranges.clear();
    app.label_hits.clear();
    app.label_targets.clear();
    app.footer_actions.clear();
    app.filter_rect = Rect::default();
    app.controls_rect = Rect::default();
    app.controls.clear();
    app.panel_regions.clear();
    app.panel_focused = true;
    let detail_area = dialogs::popup(area, 110, 32);
    app.detail_rows = detail_area.height.saturating_sub(2) as usize;
    app.detail_width = detail_area.width.saturating_sub(2);
    let child_overlay = app.update_notice.is_some()
        || app.help
        || app.settings
        || app.detail.is_some()
        || app.ui_menu.is_some()
        || app.action_menu.is_some()
        || app.prompt.is_some()
        || app.signal.is_some();
    let backdrop = app
        .main_menu
        .as_ref()
        .filter(|menu| menu.backdrop_theme == app.config.theme)
        .and_then(|menu| menu.backdrop.as_ref())
        .filter(|buffer| buffer.area == area);
    if let Some(backdrop) = backdrop {
        frame.buffer_mut().clone_from(backdrop);
    } else {
        frame.render_widget(Block::new().style(Style::new().bg(p.bg).fg(p.text)), area);
        if area.width < 60 || area.height < 18 {
            frame.render_widget(
                Paragraph::new(
                    "rstop\n\nResize to at least 60 × 18\nRecommended: 140 × 44\n\n[F10] Quit · [Esc] Menu",
                )
                .style(Style::new().fg(p.mint))
                .alignment(Alignment::Center),
                area,
            );
            return;
        }
        let buttons = footer_buttons(app, area.width, false);
        let parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(if area.height >= 28 { 3 } else { 1 }),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(footer_rows(&buttons, area.width)),
        ])
        .split(area);
        header(frame, parts[0], app, p);
        tabs(frame, parts[1], app, p);
        frame.render_widget(
            Paragraph::new(context_message(app)).style(Style::new().fg(p.muted)),
            parts[2],
        );
        app.table_rect = Rect::default();
        app.process_detail_rect = Rect::default();
        app.sort_headers.clear();
        app.network_sort_headers.clear();
        app.gpu_ranges.clear();
        match app.view {
            View::Overview => overview(frame, parts[3], app, p),
            View::Processes => {
                let body =
                    Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(parts[3]);
                process_summary(frame, body[0], app, p);
                process_table(frame, body[1], app, p);
            }
            View::Gpu => gpu_view(frame, parts[3], app, p),
            View::Storage => storage_view(frame, parts[3], app, p),
            View::Network => network_view(frame, parts[3], app, p),
            View::System => system_view(frame, parts[3], app, p),
            View::Containers => containers(frame, parts[3], app, p),
            View::Services => services(frame, parts[3], app, p),
            View::Explorer => explorer(frame, parts[3], app, p),
            View::History => history_view(frame, parts[3], app, p),
            View::Events => event_view(frame, parts[3], app, p),
        }
        controls::table_header(frame, app, p);
        footer(frame, parts[4], app, p);
        if child_overlay || app.main_menu.is_some() {
            controls::dim(frame);
        }
        if let Some(menu) = &mut app.main_menu {
            controls::dim(frame);
            menu.backdrop = Some(frame.buffer_mut().clone());
            menu.backdrop_theme = app.config.theme.clone();
        }
    }
    if app.main_menu.is_some() {
        main_menu::draw(frame, app, p);
        if child_overlay {
            controls::dim(frame);
        }
    }
    if app.help {
        app.popup_rect = help(frame, app, p);
    }
    if app.settings {
        settings(frame, app, p);
    }
    if let Some((title, body)) = &app.detail {
        let body = if app.detail_filter.is_empty() {
            body.clone()
        } else {
            body.lines()
                .filter(|line| {
                    line.to_lowercase()
                        .contains(&app.detail_filter.to_lowercase())
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let search = if app.detail_search || !app.detail_filter.is_empty() {
            format!(
                " · / {}{}",
                if app.detail_search {
                    app.editor.display(&app.detail_filter)
                } else {
                    app.detail_filter.clone()
                },
                ""
            )
        } else {
            String::new()
        };
        let follow = if app.log_target.is_some() {
            if app.follow_logs {
                " · LIVE · [f] Pause · [p] File"
            } else {
                " · PAUSED · [f] Follow · [p] File"
            }
        } else {
            ""
        };
        let title = format!("{title}{follow}{search}");
        if app.log_target.is_some() {
            app.popup_rect = log_detail(
                frame,
                &title,
                &body,
                app.detail_scroll,
                app.detail_column,
                p,
            );
        } else {
            app.popup_rect = detail(frame, &title, &body, app.detail_scroll, p);
        }
    }
    if let Some(target) = &app.signal {
        let targets = if app.signal_targets.is_empty() {
            vec![target.clone()]
        } else {
            app.signal_targets.clone()
        };
        let names = targets
            .iter()
            .map(|p| format!("  {:>8}  {}", p.pid, p.name))
            .collect::<Vec<_>>()
            .join("\n");
        let message = format!(
            "Send {:?} to {} process(es)?\n\n[↑↓] Choose signal · [PgUp/PgDn] Scroll targets\nPID and start time are rechecked for every target.\n\n[y] Confirm · [Esc] Cancel\n\n{}",
            target.signal,
            targets.len(),
            names
        );
        app.popup_rect = dialogs::notice(
            frame,
            " Confirm process signal ",
            &message,
            app.detail_scroll,
            " [↑↓] Signal · [y] Confirm · [Esc] Cancel ",
            p,
        );
    }
    if let Some((items, selected)) = &app.action_menu {
        let labels = items
            .iter()
            .map(|(label, _)| {
                (
                    label.clone(),
                    crate::management::action_key(label)
                        .map(crossterm::event::KeyCode::Char)
                        .unwrap_or(crossterm::event::KeyCode::Null),
                )
            })
            .collect::<Vec<_>>();
        let (rect, offset, outer) = menu(frame, "Workload tools", &labels, *selected, p);
        app.popup_rect = outer;
        app.action_menu_rect = rect;
        app.action_menu_offset = offset;
    }
    if let Some(m) = &mut app.ui_menu {
        let (rect, offset, outer) = menu(frame, &m.title, &m.items, m.selected, p);
        app.popup_rect = outer;
        m.rect = rect;
        m.offset = offset;
    }
    if let Some((kind, value, _)) = &app.prompt {
        if app.detail.is_some() {
            controls::dim(frame);
        }
        let value = app.editor.display(value);
        let hint = match kind.as_str() {
            "network target" => {
                "Website: https://example.com · TCP service: localhost:3000 · Enter runs a bounded connection check."
            }
            "PCAP file" => {
                "Path to an existing PCAP. Shows the first 60 packet headers without resolving names."
            }
            "folder path" => {
                "Folder to scan: up to 2 seconds / 100,000 entries. Does not delete files or follow symlinks."
            }
            "capture interface" => {
                "Choose a network interface, e.g. any (Linux), lo (Linux loopback), en0 (macOS)."
            }
            "capture filter" => {
                "Limit packets, e.g. port 443, host 127.0.0.1, or tcp. Empty captures all protocols. No shell syntax."
            }
            "capture output" => {
                "New PCAP output path. Existing files will not be overwritten. A preview follows before capture starts."
            }
            "capture confirm" => "Type yes (or y) to start capture; [Esc] cancels.",
            "log file" => "Enter the path of a regular log file to follow.",
            "I/O priority" => {
                "none · idle · best-effort 0..7 · realtime 0..7. Applies to the selected PID after Enter; existing permissions apply."
            }
            "autogroup nice" => {
                "-20..19. Affects every process in the selected process's autogroup after Enter; existing permissions apply."
            }
            "trace" | "stack" => "Type yes to attach. The selected process may briefly stop.",
            _ => "Changes apply only after Enter.",
        };
        app.popup_rect = dialogs::notice(
            frame,
            kind,
            &format!("{kind}\n\n{value}\n\n{hint}"),
            0,
            " [Enter] Continue · [Esc] Cancel ",
            p,
        );
    }
    if let Some(capture) = &app.capture
        && app
            .prompt
            .as_ref()
            .is_some_and(|(kind, _, _)| kind == "capture confirm")
    {
        app.popup_rect = dialogs::notice(
            frame,
            "CONFIRM PACKET CAPTURE",
            &capture.preview(),
            0,
            " [y] Start capture · [Esc] Cancel ",
            p,
        );
    }
    if let Some(plan) = &app.pending_plan
        && let Some((_, value, _)) = &app.prompt
    {
        app.popup_rect = dialogs::notice(
            frame,
            "Confirm workload action",
            &format!(
                "{}

> {value}▏",
                plan.preview()
            ),
            0,
            " [y] Confirm · [Esc] Cancel ",
            p,
        );
    }
    if app.update_notice.is_some() {
        updates::draw(frame, app, p);
    }
    if app.config.ascii || app.config.no_color {
        let buffer = frame.buffer_mut();
        for cell in &mut buffer.content {
            if app.config.ascii {
                let value = cell.symbol().chars().next().unwrap_or(' ');
                if !value.is_ascii() {
                    let mapped = match value {
                        '─' | '━' => '-',
                        '│' => '|',
                        '╭' | '╮' | '╰' | '╯' => '+',
                        '↑' => '^',
                        '↓' => 'v',
                        '▸' => '>',
                        '█' | '▏' | '▎' | '▍' | '▌' | '▋' | '▊' | '▉' => '#',
                        '░' => '.',
                        '┄' | '—' => '-',
                        '●' | '◉' => 'o',
                        '○' | '◇' => 'o',
                        '■' => '!',
                        '◆' => '~',
                        'Ⅱ' => '|',
                        c if (0x2800..=0x28ff).contains(&(c as u32)) => '.',
                        _ => '*',
                    };
                    cell.set_symbol(&mapped.to_string());
                }
            }
            if app.config.no_color {
                cell.fg = Color::Reset;
                cell.bg = Color::Reset;
            }
        }
    }
}

fn context_message(app: &App) -> String {
    if app.has_process_table()
        && let Some(error) = crate::filter::Query::parse(&app.filter).error
    {
        return format!(" Invalid filter: {error}");
    }
    if app.editing_filter {
        return match app.view {
            View::Network if app.network_section == 0 => {
                " Filter: port:443 · process:python · pid:42 · state:LISTEN · [↓] Results".into()
            }
            View::Network => " Filter by interface name or address · [↓] Results".into(),
            _ => " Filter: name:python · cpu:>10 · !name:rust · [↓] Results".into(),
        };
    }
    if !app.status.is_empty() {
        return format!(" {}", app.status);
    }
    crate::diagnostics::summary(app)
}
fn header(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    use crossterm::event::KeyCode;
    let mut parts = vec![];
    if app.replay {
        parts.push(("REPLAY · ".into(), None));
    }
    parts.extend([
        (
            format!("[{}]", controls::window_label(app)),
            Some(KeyCode::Char('W')),
        ),
        (" · ".into(), None),
        ("[-]".into(), Some(KeyCode::Char('-'))),
        (
            format!(" {:.2}s ", app.config.interval_ms as f64 / 1000.),
            None,
        ),
        ("[+]".into(), Some(KeyCode::Char('+'))),
        (format!(" · up {} ", duration(app.snapshot.uptime)), None),
    ]);
    let width = parts
        .iter()
        .map(|(text, _)| Line::raw(text.as_str()).width())
        .sum::<usize>()
        .min(area.width as usize) as u16;
    let mut left = vec![
        Span::styled(" rstop ", Style::new().fg(p.mint).bold()),
        Span::styled(
            format!("· {}", app.snapshot.hostname),
            Style::new().fg(p.text),
        ),
    ];
    if area.width >= 140 {
        left.push(Span::styled(
            format!(" · {}", app.snapshot.os),
            Style::new().fg(p.muted),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(left)),
        Rect::new(area.x, area.y, area.width.saturating_sub(width + 1), 1),
    );
    let mut x = area.right() - width;
    for (text, key) in parts {
        let width = (Line::raw(text.as_str()).width() as u16).min(area.right() - x);
        let rect = Rect::new(x, area.y, width, 1);
        frame.render_widget(
            Paragraph::new(text).style(if key.is_some() {
                Style::new().fg(p.blue).bold()
            } else {
                Style::new().fg(p.muted)
            }),
            rect,
        );
        if let Some(key) = key {
            app.controls.push((rect, key));
        }
        x += width;
    }
}

fn tabs(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let views = app.views();
    let labels: Vec<_> = views
        .iter()
        .map(|view| {
            format!(
                "[{}] {}",
                crate::app::commands::key_name(view.key()),
                view.name()
            )
        })
        .collect();
    let selected = views.iter().position(|v| *v == app.view).unwrap_or(0);
    let mut start = 0;
    while start < selected
        && labels[start..=selected]
            .iter()
            .map(|s| Line::raw(s.as_str()).width() + 5)
            .sum::<usize>()
            .saturating_sub(1)
            > area.width as usize
    {
        start += 1;
    }
    let mut x = area.x;
    app.tab_ranges.clear();
    for (index, view) in views.iter().enumerate().skip(start) {
        let label = &labels[index];
        let width = Line::raw(label.as_str()).width() as u16 + 4;
        if x + width > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, width, area.height);
        let active = app.view == *view;
        controls::nav_button(frame, rect, view.key(), view.name(), active, p, p.bg);
        app.tab_ranges.push((rect, *view));
        x = rect.right() + 1;
    }
}

fn footer_buttons(
    app: &App,
    width: u16,
    editing: bool,
) -> Vec<(crossterm::event::KeyCode, &'static str)> {
    use crossterm::event::KeyCode as K;
    let mut buttons = if editing {
        vec![(K::Enter, "Apply"), (K::Esc, "Cancel")]
    } else {
        let mut items = vec![(
            K::Esc,
            if app.filter.is_empty() && !app.has_row_selection() {
                "Menu"
            } else {
                "Back"
            },
        )];
        if app.can_filter() {
            items.push((K::F(3), "Filter"));
        }
        if app.has_process_table()
            || matches!(app.view, View::Network | View::Containers | View::Services)
        {
            items.push((K::F(4), "Sort"));
        }
        items.extend([(K::F(5), "View"), (K::F(6), "Tools")]);
        if app.has_process_table() || matches!(app.view, View::Services | View::Containers) {
            items.push((K::F(7), "Logs"));
        }
        items.push((K::Enter, "Details"));
        // Keep every resource action visible. Narrow terminals wrap instead of
        // silently dropping actions; extra global shortcuts use spare cells.
        items.push((K::F(10), "Quit"));
        let rows = footer_rows(&items, width);
        for extra in [
            (K::Char(':'), "Commands"),
            (K::F(1), "Help"),
            (K::F(2), "Settings"),
        ] {
            let mut candidate = items.clone();
            candidate.insert(candidate.len() - 1, extra);
            if footer_rows(&candidate, width) <= rows {
                items = candidate;
            }
        }
        items.pop(); // Quit stays last, including while editing a filter.
        items
    };
    buttons.push((K::F(10), "Quit"));
    buttons
}

fn footer_rows(buttons: &[(crossterm::event::KeyCode, &str)], width: u16) -> u16 {
    let mut rows = 1;
    let mut used = 0;
    for (key, label) in buttons {
        let cells = controls::button_width(*key, label);
        if used > 0 && used + cells > width {
            rows += 1;
            used = 0;
        }
        used += cells;
    }
    rows
}

fn footer(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let mut x = area.x;
    let mut y = area.y;
    app.footer_actions.clear();
    for (key, label) in footer_buttons(app, area.width, app.editing_filter) {
        let width = controls::button_width(key, label);
        if x + width > area.right() {
            x = area.x;
            y += 1;
        }
        let rect = Rect::new(x, y, width, 1);
        controls::key_button(frame, rect, key, label, false, p);
        app.footer_actions.push((rect, key));
        x += width;
    }
}
