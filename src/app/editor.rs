//! UTF-8 line editing shared by inline filters, log search and prompts.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct Editor {
    pub cursor: Option<usize>,
}
impl Editor {
    pub fn position(&self, text: &str) -> usize {
        let mut at = self.cursor.unwrap_or(text.len()).min(text.len());
        while !text.is_char_boundary(at) {
            at = at.saturating_sub(1);
        }
        at
    }
    pub fn reset(&mut self) {
        self.cursor = None;
    }
    pub fn display(&self, text: &str) -> String {
        let at = self.position(text);
        format!("{}▏{}", &text[..at], &text[at..])
    }
    pub fn edit(&mut self, text: &mut String, key: KeyEvent) -> bool {
        let mut at = self.position(text);
        while !text.is_char_boundary(at) {
            at = at.saturating_sub(1);
        }
        let before =
            |s: &str, pos: usize| s[..pos].char_indices().next_back().map_or(0, |(i, _)| i);
        let after =
            |s: &str, pos: usize| s[pos..].chars().next().map_or(pos, |c| pos + c.len_utf8());
        match key.code {
            KeyCode::Home => at = 0,
            KeyCode::End => at = text.len(),
            KeyCode::Left => at = before(text, at),
            KeyCode::Right => at = after(text, at),
            KeyCode::Backspace => {
                let start = before(text, at);
                text.replace_range(start..at, "");
                at = start;
            }
            KeyCode::Delete => {
                text.replace_range(at..after(text, at), "");
            }
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => match c {
                'a' => at = 0,
                'e' => at = text.len(),
                'u' => {
                    text.drain(..at);
                    at = 0;
                }
                'k' => {
                    text.truncate(at);
                }
                'w' => {
                    let end = at;
                    while at > 0
                        && text[..at]
                            .chars()
                            .next_back()
                            .is_some_and(char::is_whitespace)
                    {
                        at = before(text, at);
                    }
                    while at > 0
                        && text[..at]
                            .chars()
                            .next_back()
                            .is_some_and(|c| !c.is_whitespace())
                    {
                        at = before(text, at);
                    }
                    text.replace_range(at..end, "");
                }
                _ => return false,
            },
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::ALT | KeyModifiers::CONTROL) =>
            {
                text.insert(at, c);
                at += c.len_utf8();
            }
            _ => return false,
        }
        self.cursor = Some(at);
        true
    }
}
