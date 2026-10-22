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
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let s = self.active.as_ref()?;
        if s.anchor == s.head {
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
        if s.anchor == s.head {
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
                    buffer[(column, y)] = s.region.buffer[(column, y)].clone();
                    if selected {
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
