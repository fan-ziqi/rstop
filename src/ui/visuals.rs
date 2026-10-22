use super::{Palette, block};
use ratatui::{prelude::*, widgets::*};

/// A fixed-scale track. Missing counters have no fill; they are never drawn as zero.
pub(super) fn track(
    value: Option<f64>,
    max: f64,
    width: usize,
    color: Color,
    p: Palette,
) -> Vec<Span<'static>> {
    let Some(value) = value
        .filter(|v| v.is_finite())
        .filter(|_| max.is_finite() && max > 0.)
    else {
        return vec![Span::styled("┄".repeat(width), Style::new().fg(p.border))];
    };
    // Filled tracks occupy more area than labels; keep their emphasis lower.
    let color = match (p.panel, color) {
        (Color::Rgb(r, g, b), Color::Rgb(cr, cg, cb)) => {
            let blend =
                |base: u8, accent: u8| ((u16::from(base) + u16::from(accent) * 4) / 5) as u8;
            Color::Rgb(blend(r, cr), blend(g, cg), blend(b, cb))
        }
        _ => color,
    };
    let units = (value / max).clamp(0., 1.) * width as f64;
    let whole = units.floor() as usize;
    let fraction = ((units - whole as f64) * 8.).floor() as usize;
    let mut spans = vec![Span::styled("█".repeat(whole), Style::new().fg(color))];
    let partial = usize::from(whole < width && fraction > 0);
    if partial > 0 {
        spans.push(Span::styled(
            ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][fraction],
            Style::new().fg(color),
        ));
    }
    spans.push(Span::styled(
        "░".repeat(width.saturating_sub(whole + partial)),
        Style::new().fg(p.border),
    ));
    spans
}

pub(super) fn metric_cell(
    label: String,
    value: Option<f64>,
    color: Color,
    p: Palette,
) -> Cell<'static> {
    Cell::from(label).style(Style::new().fg(if value.is_some_and(|v| v.is_finite()) {
        color
    } else {
        p.muted
    }))
}

pub(super) fn state_cell(state: &str, p: Palette) -> Cell<'static> {
    let color = match state.to_ascii_lowercase().as_str() {
        "running" | "active" | "up" | "established" | "run" | "runnable" | "healthy" => p.mint,
        "failed" | "dead" | "unhealthy" | "zombie" => p.red,
        "restarting" | "activating" | "deactivating" | "time_wait" | "close_wait" | "paused"
        | "idle" => p.amber,
        "listen" | "listening" => p.blue,
        _ => p.muted,
    };
    Cell::from(state.to_string()).style(Style::new().fg(color))
}

pub(super) struct Stat<'a> {
    pub label: &'a str,
    pub value: String,
    pub color: Color,
    pub percent: Option<f64>,
}

pub(super) fn stats(frame: &mut Frame, area: Rect, items: &[Stat<'_>], p: Palette) {
    if items.is_empty() || area.is_empty() {
        return;
    }
    let columns = Layout::horizontal(vec![Constraint::Ratio(1, items.len() as u32); items.len()])
        .spacing(1)
        .split(area);
    for (item, area) in items.iter().zip(columns.iter()) {
        let b = block(format!(" {} ", item.label), p, item.color);
        let inner = b.inner(*area);
        frame.render_widget(b, *area);
        frame.render_widget(
            Paragraph::new(item.value.clone()).style(Style::new().fg(p.text).bold()),
            Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
        );
        if inner.height >= 2 && item.percent.is_some() {
            frame.render_widget(
                Paragraph::new(Line::from(track(
                    item.percent,
                    100.,
                    inner.width as usize,
                    item.color,
                    p,
                ))),
                Rect::new(inner.x, inner.y + 1, inner.width, 1),
            );
        }
    }
}

pub(super) fn field(label: &str, value: impl Into<String>, p: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label:<10}"), Style::new().fg(p.muted)),
        Span::styled(value.into(), Style::new().fg(p.text)),
    ])
}

pub(super) fn empty(frame: &mut Frame, area: Rect, title: &str, hint: &str, p: Palette) {
    if area.is_empty() {
        return;
    }
    let lines = vec![
        Line::styled(format!("◇ {title}"), Style::new().fg(p.muted).bold()),
        Line::styled(hint.to_string(), Style::new().fg(p.muted)),
    ];
    let height = area.height.min(2);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        Rect::new(
            area.x,
            area.y + (area.height - height) / 2,
            area.width,
            height,
        ),
    );
}
