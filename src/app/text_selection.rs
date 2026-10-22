//! Selection uses a frozen copy of the visible text, so live samples cannot
//! change the text between dragging and copying.
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Modifier,
    text::Line,
};

#[derive(Clone)]
pub struct TextRegion {
    pub id: String,
    pub offset: (usize, u16),
    pub buffer: Buffer,
}
impl TextRegion {
    pub fn is_screen(&self) -> bool {
        self.id.starts_with("screen:")
    }
    pub fn capture(id: String, offset: (usize, u16), area: Rect, source: &Buffer) -> Self {
        let mut buffer = Buffer::empty(area);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buffer[(x, y)] = source[(x, y)].clone();
            }
        }
        Self { id, offset, buffer }
    }
}

#[derive(Default)]
pub struct TextSelection {
    active: Option<Selection>,
}
struct Selection {
    region: TextRegion,
    anchor: Position,
    head: Position,
    dragging: bool,
    selected: bool,
}
impl TextSelection {
    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn begin(&mut self, region: TextRegion, position: Position) {
        self.active = Some(Selection {
            region,
            anchor: position,
            head: position,
            dragging: true,
            selected: false,
        });
    }

    pub fn dragging(&self) -> bool {
        self.active.as_ref().is_some_and(|s| s.dragging)
    }

    pub fn extend(&mut self, position: Position, finish: bool) {
        if let Some(s) = &mut self.active {
            let area = s.region.buffer.area;
            s.head = Position::new(
                position.x.clamp(area.x, area.right() - 1),
                position.y.clamp(area.y, area.bottom() - 1),
            );
            s.dragging = !finish;
            s.selected |= !finish || s.head != s.anchor;
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let s = self.active.as_ref()?;
        if !s.selected {
            return None;
        }
        let (start, end) = s.bounds();
        let area = s.region.buffer.area;
        let mut rows = vec![];
        for y in start.y..=end.y {
            let left = if y == start.y { start.x } else { area.x };
            let right = if y == end.y { end.x + 1 } else { area.right() };
            let mut row = String::new();
            let mut x = area.x;
            while x < area.right() {
                let symbol = s.region.buffer[(x, y)].symbol();
                let width = Line::raw(symbol).width().max(1) as u16;
                // Include a whole grapheme even when an endpoint is its second cell.
                if x < right && x + width > left {
                    row.push_str(symbol);
                }
                x += width;
            }
            rows.push(row.trim_end().to_owned());
        }
        let text = rows.join("\n");
        (!text.trim().is_empty()).then_some(text)
    }

    pub fn render(&mut self, buffer: &mut Buffer, regions: &[TextRegion]) {
        let Some(s) = &self.active else {
            return;
        };
        if !regions.iter().any(|r| {
            r.id == s.region.id
                && r.offset == s.region.offset
                && r.buffer.area == s.region.buffer.area
        }) {
            self.clear();
            return;
        }
        if !s.selected {
            return;
        }
        let (start, end) = s.bounds();
        let area = s.region.buffer.area;
        for y in area.y..area.bottom() {
            let mut x = area.x;
            while x < area.right() {
                let cell = &s.region.buffer[(x, y)];
                let width = Line::raw(cell.symbol()).width().max(1) as u16;
                let selected = y >= start.y
                    && y <= end.y
                    && (y != start.y || x + width > start.x)
                    && (y != end.y || x <= end.x);
                for column in x..(x + width).min(area.right()) {
                    if selected {
                        buffer[(column, y)] = s.region.buffer[(column, y)].clone();
                        buffer[(column, y)].set_style(
                            ratatui::style::Style::new().add_modifier(Modifier::REVERSED),
                        );
                    }
                }
                x += width;
            }
        }
    }
}
impl Selection {
    fn bounds(&self) -> (Position, Position) {
        if (self.anchor.y, self.anchor.x) <= (self.head.y, self.head.x) {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn screen_selection_freezes_selected_cells_while_other_values_keep_refreshing() {
        let area = Rect::new(0, 0, 12, 1);
        let mut old = Buffer::empty(area);
        old.set_string(0, 0, "old 42", Style::default());
        let mut selection = TextSelection::default();
        selection.begin(
            TextRegion::capture("screen:test".into(), (0, 0), area, &old),
            (0, 0).into(),
        );
        selection.extend((2, 0).into(), true);
        let mut live = Buffer::empty(area);
        live.set_string(0, 0, "new 99", Style::default());
        let regions = [TextRegion::capture(
            "screen:test".into(),
            (0, 0),
            area,
            &live,
        )];
        selection.render(&mut live, &regions);
        assert_eq!(selection.selected_text().as_deref(), Some("old"));
        assert_eq!(
            (0..6).map(|x| live[(x, 0)].symbol()).collect::<String>(),
            "old 99"
        );
    }

    #[test]
    fn a_single_cell_drag_is_copyable_but_a_click_is_not_a_selection() {
        let area = Rect::new(0, 0, 4, 1);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "0", Style::default());
        let mut selection = TextSelection::default();
        selection.begin(
            TextRegion::capture("screen:test".into(), (0, 0), area, &buffer),
            (0, 0).into(),
        );
        selection.extend((0, 0).into(), true);
        assert!(selection.selected_text().is_none());
        selection.extend((0, 0).into(), false);
        selection.extend((0, 0).into(), true);
        assert_eq!(selection.selected_text().as_deref(), Some("0"));
    }
}
