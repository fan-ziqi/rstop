use anyhow::Result;
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use rstop::{app::App, ui};
use std::path::Path;

pub fn svg(path: &Path, width: u16, height: u16, app: &mut App) -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| ui::draw(frame, app))?;
    let buffer = terminal.backend().buffer();
    let p = ui::palette(&app.config.theme);
    let color = |color: Color| -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => "#d7e2f1".into(),
        }
    };
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let w = width as u32 * 8 + 32;
    let h = height as u32 * 18 + 32;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><rect width=\"100%\" height=\"100%\" fill=\"{}\"/><g font-family=\"DejaVu Sans Mono,monospace\" font-size=\"13\" xml:space=\"preserve\">",
        color(p.bg)
    );
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let px = 16 + x as u32 * 8;
            let py = 16 + y as u32 * 18;
            svg.push_str(&format!(
                "<rect x=\"{px}\" y=\"{py}\" width=\"8\" height=\"18\" fill=\"{}\"/>",
                color(if cell.bg == Color::Reset {
                    p.bg
                } else {
                    cell.bg
                })
            ));
        }
    }
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            let px = 16 + x as u32 * 8;
            let py = 16 + y as u32 * 18;
            if !symbol.trim().is_empty() {
                let fg = color(cell.fg);
                let glyph = symbol.chars().next().unwrap_or(' ');
                let code = glyph as u32;
                if (0x2800..=0x28ff).contains(&code) {
                    let dots = code - 0x2800;
                    for (bit, dx, dy) in [
                        (0, 2, 2),
                        (1, 2, 6),
                        (2, 2, 10),
                        (3, 6, 2),
                        (4, 6, 6),
                        (5, 6, 10),
                        (6, 2, 14),
                        (7, 6, 14),
                    ] {
                        if dots & (1 << bit) != 0 {
                            svg.push_str(&format!(
                                "<circle cx=\"{}\" cy=\"{}\" r=\"0.85\" fill=\"{fg}\"/>",
                                px + dx,
                                py + dy
                            ));
                        }
                    }
                    continue;
                }
                let line = match glyph {
                    '─' | '━' => Some("M0 9 H8"),
                    '│' => Some("M4 0 V18"),
                    '╭' => Some("M4 18 V13 Q4 9 8 9"),
                    '╮' => Some("M0 9 Q4 9 4 13 V18"),
                    '╰' => Some("M4 0 V5 Q4 9 8 9"),
                    '╯' => Some("M4 0 V5 Q4 9 0 9"),
                    _ => None,
                };
                if let Some(line) = line {
                    svg.push_str(&format!("<path transform=\"translate({px} {py})\" d=\"{line}\" fill=\"none\" stroke=\"{fg}\" stroke-width=\"{}\"/>",if glyph=='━'{2.0}else{1.0}));
                    continue;
                }
                svg.push_str(&format!(
                    "<text x=\"{px}\" y=\"{}\" fill=\"{}\"{}>{}</text>",
                    py + 14,
                    color(cell.fg),
                    if cell.modifier.contains(Modifier::BOLD) {
                        " font-weight=\"bold\""
                    } else {
                        ""
                    },
                    escape(symbol)
                ));
            }
        }
    }
    svg.push_str("</g></svg>");
    std::fs::write(path, svg)?;
    Ok(())
}
