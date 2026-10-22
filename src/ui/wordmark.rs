//! Static block faces with an outlined extrusion, following btop's banner treatment.
//! https://github.com/aristocratos/btop/blob/main/src/btop_draw.cpp
use super::Palette;
use ratatui::{prelude::*, widgets::Paragraph};

pub(super) const HEIGHT: u16 = 6;
const ASCII_LOGO: [&str; 6] = [
    r"             _",
    r" _ __  ___  | |_    ___   _ __",
    r"| '__|/ __| | __|  / _ \ | '_ \",
    r"| |   \__ \ | |_  | (_) || |_) |",
    r"|_|   |___/  \__|  \___/ | .__/",
    r"                         |_|",
];

const LOGO: [&str; 6] = [
    "██████╗ ███████╗████████╗ ██████╗ ██████╗ ",
    "██╔══██╗██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗",
    "██████╔╝███████╗   ██║   ██║   ██║██████╔╝",
    "██╔══██╗╚════██║   ██║   ██║   ██║██╔═══╝ ",
    "██║  ██║███████║   ██║   ╚██████╔╝██║     ",
    "╚═╝  ╚═╝╚══════╝   ╚═╝    ╚═════╝ ╚═╝     ",
];

pub(super) fn draw(frame: &mut Frame, area: Rect, ascii: bool, p: Palette) {
    let rows = if ascii { &ASCII_LOGO } else { &LOGO };
    let width = rows
        .iter()
        .map(|row| Line::raw(*row).width() as u16)
        .max()
        .unwrap_or(0);
    let x = area.x + area.width.saturating_sub(width) / 2;
    for (row, text) in rows.iter().take(area.height as usize).enumerate() {
        let spans = text
            .chars()
            .map(|ch| {
                Span::styled(
                    ch.to_string(),
                    Style::new().fg(if ascii {
                        p.text
                    } else if ch == '█' {
                        p.mint
                    } else {
                        p.muted
                    }),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(x, area.y + row as u16, width.min(area.width), 1),
        );
    }
}
