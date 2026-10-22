use crossterm::event::KeyCode;
use ratatui::layout::Rect;

#[derive(Default)]
pub struct State {
    pub scroll: usize,
    pub rows: usize,
    pub lines: usize,
    pub content: Rect,
    pub input: Rect,
    pub error: String,
    pub settings_rows: Vec<(Rect, usize)>,
    pub settings_adjust: Vec<(Rect, usize, KeyCode)>,
    pub settings_message: String,
}

impl State {
    pub fn reset_geometry(&mut self) {
        self.content = Rect::default();
        self.input = Rect::default();
        self.settings_rows.clear();
        self.settings_adjust.clear();
    }
}
