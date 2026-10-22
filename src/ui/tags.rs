use super::Palette;
use crate::tags::{Confidence, Tag};
use ratatui::{prelude::*, widgets::Cell};

/// One plan supplies both terminal cells and mouse geometry. Nothing is cut mid-badge.
pub(super) struct BadgePlan {
    pub line: Line<'static>,
    ranges: Vec<(u16, u16)>,
}
impl BadgePlan {
    pub fn cell(&self) -> Cell<'static> {
        Cell::from(self.line.clone())
    }
    pub fn hits(&self, x: u16, y: u16) -> Vec<Rect> {
        self.ranges
            .iter()
            .map(|(offset, width)| Rect::new(x + offset, y, *width, 1))
            .collect()
    }
}

/// Terminal-cell, grapheme-safe truncation, shared by object names and popup titles.
pub(super) fn elide(text: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let clean: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if Line::raw(&clean).width() <= width as usize {
        return clean;
    }
    let mut result = String::new();
    let mut used = 0;
    let span = Span::raw(&clean);
    for grapheme in span.styled_graphemes(Style::default()) {
        let cells = Line::raw(grapheme.symbol).width();
        if used + cells > width.saturating_sub(1) as usize {
            break;
        }
        result.push_str(grapheme.symbol);
        used += cells;
    }
    result.push('…');
    result
}

fn chip_style(tag: Option<&Tag>, p: Palette, no_color: bool) -> Style {
    let color = match tag {
        None => p.muted,
        Some(tag) if tag.confidence == Confidence::Inferred => p.amber,
        Some(tag)
            if ["failed", "unhealthy", "restarting", "zombie"].contains(&tag.key.as_str()) =>
        {
            p.red
        }
        Some(tag)
            if [
                "gpu-active",
                "gpu-memory",
                "cpu-cores",
                "memory-heavy",
                "disk-io",
                "network-io",
            ]
            .contains(&tag.key.as_str()) =>
        {
            p.purple
        }
        Some(_) => p.blue,
    };
    if no_color {
        Style::new().remove_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        // Give roles enough hue to distinguish them without bright backgrounds.
        Style::new()
            .fg(blend(p.muted, color, 60))
            .bg(blend(p.panel, color, 9))
            .remove_modifier(Modifier::BOLD | Modifier::REVERSED)
    }
}

fn blend(base: Color, accent: Color, percent: u16) -> Color {
    match (base, accent) {
        (Color::Rgb(r, g, b), Color::Rgb(ar, ag, ab)) => {
            let channel = |a: u8, b: u8| {
                ((u16::from(a) * (100 - percent) + u16::from(b) * percent) / 100) as u8
            };
            Color::Rgb(channel(r, ar), channel(g, ag), channel(b, ab))
        }
        // Respect custom terminal/indexed palettes without reintroducing bright accent fills.
        _ => base,
    }
}

pub(super) struct TagRow {
    pub plan: BadgePlan,
    pub keys: Vec<String>,
}
fn empty_row() -> TagRow {
    TagRow {
        plan: BadgePlan {
            line: Line::default(),
            ranges: vec![],
        },
        keys: vec![],
    }
}

/// Complete detail badges wrap as units. An unusually long label wraps by grapheme.
pub(super) fn full_rows(tags: &[Tag], width: u16, p: Palette, no_color: bool) -> Vec<TagRow> {
    if tags.is_empty() || width < 2 {
        return vec![];
    }
    let mut rows = vec![];
    let mut row = empty_row();
    let mut used = 0u16;
    for tag in tags {
        let text = format!(
            " {} ",
            tag.display()
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>()
        );
        let mut chunks = vec![];
        let mut chunk = String::new();
        let mut chunk_width = 0u16;
        let span = Span::raw(text);
        for g in span.styled_graphemes(Style::default()) {
            let cells = Line::raw(g.symbol).width() as u16;
            if chunk_width + cells > width && !chunk.is_empty() {
                chunks.push(std::mem::take(&mut chunk));
                chunk_width = 0;
            }
            chunk.push_str(g.symbol);
            chunk_width += cells;
        }
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        for (index, chunk) in chunks.iter().enumerate() {
            let cells = Line::raw(chunk).width() as u16;
            if used > 0 && (used + 1 + cells > width || chunks.len() > 1) {
                rows.push(std::mem::replace(&mut row, empty_row()));
                used = 0;
            }
            if used > 0 {
                row.plan.line.spans.push(Span::raw(" "));
                used += 1;
            }
            row.plan.ranges.push((used, cells));
            row.keys.push(tag.key.clone());
            row.plan.line.spans.push(Span::styled(
                chunk.clone(),
                chip_style(Some(tag), p, no_color),
            ));
            used += cells;
            if index + 1 < chunks.len() {
                rows.push(std::mem::replace(&mut row, empty_row()));
                used = 0;
            }
        }
    }
    if !row.keys.is_empty() {
        rows.push(row);
    }
    rows
}

fn overflow(count: usize, budget: u16) -> String {
    let label = format!("+{count}");
    if label.len() + 2 <= budget as usize {
        format!(" {label} ")
    } else if label.len() <= budget as usize {
        label
    } else if budget > 0 {
        "+".into()
    } else {
        String::new()
    }
}

pub(super) fn plan(
    tags: &[Tag],
    content: String,
    width: u16,
    p: Palette,
    no_color: bool,
) -> BadgePlan {
    let content_width = Line::raw(&content).width().min(u16::MAX as usize) as u16;
    // Identity gets at least twelve cells or 60% of a narrow identity column.
    let reserve = content_width.min(((width as u32 * 3 / 5) as u16).max(12).min(width));
    let budget = width.saturating_sub(reserve + u16::from(reserve > 0));
    let labels: Vec<_> = tags
        .iter()
        .map(|tag| format!(" {} ", tag.display()))
        .collect();
    let mut shown = 0;
    let mut used = 0u16;
    for label in &labels {
        let cells = Line::raw(label).width().min(u16::MAX as usize) as u16;
        let next = used
            .saturating_add(u16::from(shown > 0))
            .saturating_add(cells);
        let hidden = tags.len() - shown - 1;
        let tail = if hidden > 0 {
            1 + overflow(hidden, budget).len() as u16
        } else {
            0
        };
        if next.saturating_add(tail) > budget {
            break;
        }
        used = next;
        shown += 1;
    }
    let hidden = tags.len() - shown;
    let more = if hidden > 0 {
        overflow(hidden, budget.saturating_sub(used + u16::from(shown > 0)))
    } else {
        String::new()
    };
    let badge_width = used
        + if more.is_empty() {
            0
        } else {
            u16::from(shown > 0) + more.len() as u16
        };
    let gap = u16::from(content_width > 0 && badge_width > 0);
    let name = elide(&content, width.saturating_sub(badge_width + gap));
    let mut offset = Line::raw(&name).width() as u16;
    let mut spans = vec![Span::styled(name, Style::new().fg(p.text))];
    let mut ranges = vec![];
    if gap > 0 {
        spans.push(Span::raw(" "));
        offset += 1;
    }
    for (i, label) in labels.into_iter().take(shown).enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
            offset += 1;
        }
        let cells = Line::raw(&label).width() as u16;
        ranges.push((offset, cells));
        offset += cells;
        let mut style = chip_style(tags.get(i), p, no_color);
        style.bg = None;
        spans.push(Span::styled(label, style));
    }
    if !more.is_empty() {
        if shown > 0 {
            spans.push(Span::raw(" "));
            offset += 1;
        }
        ranges.push((offset, more.len() as u16));
        let mut style = chip_style(None, p, no_color);
        style.bg = None;
        spans.push(Span::styled(more, style));
    }
    BadgePlan {
        line: Line::from(spans),
        ranges,
    }
}

/// Ratatui reserves this exact area for the highlight symbol, including on empty tables.
pub(super) fn columns(inner: Rect, widths: &[Constraint], highlight: u16) -> Vec<Rect> {
    Layout::horizontal(widths.to_vec())
        .spacing(1)
        .flex(layout::Flex::Start)
        .split(Rect::new(
            inner.x + highlight.min(inner.width),
            inner.y,
            inner.width.saturating_sub(highlight),
            1,
        ))
        .to_vec()
}

pub(super) fn column_width(width: u16) -> u16 {
    if width < 80 {
        11
    } else if width < 120 {
        18
    } else {
        24
    }
}

/// Keep label colors and click geometry while inheriting the table's row background.
pub(super) fn render_column(
    frame: &mut Frame,
    plans: &[BadgePlan],
    column: Rect,
    rows: Rect,
    offset: usize,
) {
    for (i, plan) in plans
        .iter()
        .skip(offset)
        .take(rows.height as usize)
        .enumerate()
    {
        let y = rows.y + i as u16;
        frame.render_widget(
            ratatui::widgets::Paragraph::new(plan.line.clone()),
            Rect::new(column.x, y, column.width, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_badges_wrap_without_losing_text_or_their_action_targets() {
        let tags: Vec<_> = [
            "Web",
            "数据库👩‍💻 e\u{301} with a very long label",
            "GPU memory",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| Tag {
            key: i.to_string(),
            label: (*label).into(),
            ..Default::default()
        })
        .collect();
        for width in 2..80 {
            let rows = full_rows(&tags, width, super::super::palette("default"), false);
            let mut restored = std::collections::HashMap::<String, String>::new();
            for row in rows {
                assert!(row.plan.line.width() <= width as usize);
                assert_eq!(row.plan.hits(0, 0).len(), row.keys.len());
                for (span, key) in row
                    .plan
                    .line
                    .spans
                    .iter()
                    .filter(|s| s.style.bg.is_some())
                    .zip(&row.keys)
                {
                    restored
                        .entry(key.clone())
                        .or_default()
                        .push_str(&span.content);
                }
            }
            for tag in &tags {
                assert_eq!(restored[&tag.key], format!(" {} ", tag.display()));
            }
        }
    }
    #[test]
    fn badges_preserve_identity_and_account_for_every_hidden_label() {
        let p = super::super::palette("default");
        let tags: Vec<_> = (0..50)
            .map(|i| Tag {
                key: format!("role{i}"),
                label: format!("Label {i}"),
                ..Default::default()
            })
            .collect();
        for width in 0..180 {
            let plan = plan(
                &tags,
                "数据库👩‍💻 e\u{301} long process name".into(),
                width,
                p,
                false,
            );
            assert!(
                plan.line.width() <= width as usize,
                "{width}: {}",
                plan.line
            );
            for hit in plan.hits(5, 4) {
                assert!(hit.x >= 5 && hit.right() <= 5 + width);
            }
            if width >= 20 {
                assert!(plan.line.to_string().contains('+'));
            }
            assert!(!plan.line.to_string().contains('['));
        }
        let full = plan(&tags[..1], "short".into(), 50, p, false);
        assert_eq!(full.line.to_string(), "short  Label 0 ");
        assert_eq!(full.hits(0, 0), vec![Rect::new(6, 0, 9, 1)]);
        assert!(full.line.spans[2].style.bg.is_none());
        let mono = plan(&tags[..1], String::new(), 50, p, true);
        assert!(
            mono.line
                .spans
                .iter()
                .all(|s| !s.style.add_modifier.contains(Modifier::DIM))
        );
        assert!(full.line.spans.iter().all(|s| {
            !s.style
                .add_modifier
                .intersects(Modifier::BOLD | Modifier::REVERSED)
        }));
    }
}
