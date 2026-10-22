use super::{Palette, controls, dialogs};
use crate::{app::App, updates::Phase};
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};

pub(super) fn draw(frame: &mut Frame, app: &mut App, p: Palette) {
    let Some(notice) = &app.update_notice else {
        return;
    };
    let stacked = frame.area().width < 82;
    let (status, color) = match &notice.phase {
        Phase::Available => ("A new version is available.".into(), p.muted),
        Phase::Installing => (
            "Upgrading in the background. You can close this window.".into(),
            p.mint,
        ),
        Phase::Installed(version) => (
            format!("Updated to {version}. Restart rstop to use the new version."),
            p.mint,
        ),
        Phase::Failed(error) => (
            error
                .chars()
                .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                .collect(),
            p.red,
        ),
    };
    let buttons = match &notice.phase {
        Phase::Installing => vec![(KeyCode::Esc, "Close", 1)],
        Phase::Installed(_) => vec![(KeyCode::Enter, "Continue monitoring", 1)],
        Phase::Failed(_) => vec![
            (KeyCode::Char('u'), "Retry", 0),
            (KeyCode::Char('i'), "Close", 1),
        ],
        _ => vec![
            (KeyCode::Char('u'), "Upgrade", 0),
            (KeyCode::Char('i'), "Ignore this session", 1),
            (KeyCode::Char('n'), "Skip this version", 2),
        ],
    };
    let actions_height = if stacked {
        buttons.len() as u16 * 2 - 1
    } else {
        3
    };
    let message_rows = Paragraph::new(status.as_str())
        .wrap(Wrap { trim: false })
        .line_count(frame.area().width.min(84).saturating_sub(10).max(1));
    let height = (actions_height as usize + message_rows.max(2) + 9).min(24) as u16;
    let area = dialogs::popup(frame.area(), 84, height);
    app.popup_rect = area;
    app.popup_actions.clear();
    let inner = dialogs::surface(frame, area, "rstop update", p, p.mint);
    let message_rows = Paragraph::new(status.as_str())
        .wrap(Wrap { trim: false })
        .line_count(inner.width.max(1));
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(actions_height),
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
        Paragraph::new("[r] View release notes").style(Style::new().fg(p.blue).underlined()),
        rows[2],
    );
    app.update_buttons.push((rows[2], 3));
    let mut message = rows[4];
    if message_rows > message.height as usize {
        message.height = message.height.saturating_sub(1).max(1);
    }
    app.dialog.rows = message.height as usize;
    app.dialog.lines = message_rows;
    app.dialog.content = message;
    app.dialog.scroll = app
        .dialog
        .scroll
        .min(message_rows.saturating_sub(message.height as usize));
    frame.render_widget(
        Paragraph::new(status)
            .wrap(Wrap { trim: false })
            .scroll((app.dialog.scroll.min(u16::MAX as usize) as u16, 0))
            .style(Style::new().fg(color)),
        message,
    );
    if message_rows > message.height as usize {
        frame.render_widget(
            Paragraph::new(format!(
                "[PgUp/PgDn] Message  {}–{}/{}",
                app.dialog.scroll + 1,
                (app.dialog.scroll + message.height as usize).min(message_rows),
                message_rows
            ))
            .style(Style::new().fg(p.muted)),
            Rect::new(rows[4].x, rows[4].bottom() - 1, rows[4].width, 1),
        );
    }
    let cells = if stacked {
        buttons
            .iter()
            .enumerate()
            .map(|(index, _)| Rect::new(rows[6].x, rows[6].y + index as u16 * 2, rows[6].width, 1))
            .collect::<Vec<_>>()
    } else {
        Layout::horizontal(vec![
            Constraint::Ratio(1, buttons.len() as u32);
            buttons.len()
        ])
        .split(rows[6])
        .to_vec()
    };
    for ((key, label, index), rect) in buttons.iter().zip(cells) {
        controls::nav_button(
            frame,
            rect,
            *key,
            label,
            *index == notice.selected || buttons.len() == 1,
            p,
            p.panel,
        );
        app.update_buttons.push((rect, *index));
    }
}
