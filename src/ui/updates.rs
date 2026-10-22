use super::{Palette, block, controls, dialogs};
use crate::{app::App, updates::Phase};
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};

pub(super) fn draw(frame: &mut Frame, app: &mut App, p: Palette) {
    let Some(notice) = &app.update_notice else {
        return;
    };
    let area = dialogs::popup(frame.area(), 84, 12);
    app.popup_rect = area;
    frame.render_widget(Clear, area);
    let border = block(" rstop update ", p, p.mint).style(Style::new().bg(p.bg));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(
            "{}  →  {}",
            env!("CARGO_PKG_VERSION"),
            notice.release.version()
        ))
        .style(Style::new().fg(p.text).bold()),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(format!("[r] {}", notice.release.html_url))
            .style(Style::new().fg(p.blue).underlined())
            .wrap(Wrap { trim: false }),
        rows[1],
    );
    app.update_buttons.push((rows[1], 3));
    let (status, color) = match &notice.phase {
        Phase::Available => ("New version available".into(), p.muted),
        Phase::Installing => ("Upgrading…".into(), p.mint),
        Phase::Installed(version) => (
            format!("Updated to {version}. Restart rstop to use the new version."),
            p.mint,
        ),
        Phase::Failed(error) => (
            error
                .chars()
                .filter(|c| !c.is_control())
                .take(300)
                .collect(),
            p.red,
        ),
    };
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::new().fg(color))
            .wrap(Wrap { trim: false }),
        rows[2],
    );
    if matches!(notice.phase, Phase::Installing) {
        return;
    }
    if matches!(notice.phase, Phase::Installed(_)) {
        controls::nav_button(
            frame,
            rows[3],
            KeyCode::Enter,
            "Continue monitoring",
            true,
            p,
            p.bg,
        );
        app.update_buttons.push((rows[3], 1));
        return;
    }
    let buttons = if matches!(notice.phase, Phase::Failed(_)) {
        vec![(KeyCode::Char('u'), "Retry"), (KeyCode::Char('i'), "Close")]
    } else {
        let (ignore, skip) = if area.width < 78 {
            ("Ignore once", "Skip version")
        } else {
            ("Ignore this session", "Skip this version")
        };
        vec![
            (KeyCode::Char('u'), "Upgrade"),
            (KeyCode::Char('i'), ignore),
            (KeyCode::Char('n'), skip),
        ]
    };
    let cells = Layout::horizontal(vec![
        Constraint::Ratio(1, buttons.len() as u32);
        buttons.len()
    ])
    .split(rows[3]);
    for (index, ((key, label), rect)) in buttons.iter().zip(cells.iter()).enumerate() {
        controls::nav_button(frame, *rect, *key, label, index == notice.selected, p, p.bg);
        app.update_buttons.push((*rect, index));
    }
}
