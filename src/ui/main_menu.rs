use super::{
    Palette,
    controls::nav_button,
    dialogs::{self, popup},
    wordmark,
};
use crate::{
    app::{
        App,
        menu::{MenuAction, ProjectLink},
    },
    project,
};
use ratatui::{prelude::*, widgets::*};

fn blend(a: Color, b: Color, t: f64) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let mix = |a: u8, b: u8| (a as f64 * (1.0 - t) + b as f64 * t) as u8;
            Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
        }
        _ => a,
    }
}

pub(super) fn draw(frame: &mut Frame, app: &mut App, p: Palette) {
    let menu = app.main_menu.as_mut().expect("main menu is open");
    menu.hitboxes.clear();
    menu.links.clear();
    let area = popup(frame.area(), 78, 32);
    app.popup_rect = area;
    dialogs::surface(frame, area, "rstop", p, p.mint);
    let inner = area.inner(Margin::new(2, 1));
    let roomy = inner.height >= 30 && inner.width >= 54;
    let count = MenuAction::ALL.len() as u16;
    let logo_height = wordmark::HEIGHT;
    let logo_y = inner.y + u16::from(roomy);
    wordmark::draw(
        frame,
        Rect::new(inner.x, logo_y, inner.width, logo_height),
        app.config.ascii,
        p,
    );
    let intro_y = logo_y + logo_height + u16::from(roomy);
    let show_intro = inner.height >= logo_height + count + 5;
    if show_intro {
        frame.render_widget(
            Paragraph::new("One terminal. Your whole system.")
                .alignment(Alignment::Center)
                .style(Style::new().fg(p.text)),
            Rect::new(inner.x, intro_y, inner.width, 1),
        );
    }
    let meta_y = intro_y + if roomy { 2 } else { u16::from(show_intro) };
    let developer = format!("{}  ·  ", project::DEVELOPER);
    let metadata = format!(
        "{developer}v{}  ·  {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_LICENSE")
    );
    let width = Line::raw(&metadata).width() as u16;
    let x = inner.x + (inner.width - width) / 2;
    frame.render_widget(
        Paragraph::new(metadata).style(Style::new().fg(p.muted)),
        Rect::new(x, meta_y, width, 1),
    );
    menu.links.push((
        Rect::new(x, meta_y, project::DEVELOPER.len() as u16, 1),
        ProjectLink::Developer,
    ));
    for (index, link) in [ProjectLink::Source, ProjectLink::Coffee]
        .into_iter()
        .enumerate()
    {
        let label = link
            .url()
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let color = if link == ProjectLink::Source {
            blend(p.blue, p.muted, 0.35)
        } else {
            blend(p.amber, p.muted, 0.65)
        };
        let width = label.len() as u16;
        let rect = Rect::new(
            inner.x + (inner.width - width) / 2,
            meta_y + 1 + index as u16,
            width,
            1,
        );
        frame.render_widget(Paragraph::new(label).style(Style::new().fg(color)), rect);
        menu.links.push((rect, link));
    }
    let content_end = meta_y + 3;
    let start = content_end + if roomy { 2 } else { 0 };
    let available = (inner.bottom() - 1).saturating_sub(start);
    let height = if available >= count * 3 { 3 } else { 1 };
    let gap = u16::from(available >= height * count + count - 1);
    let menu_height = height * count + gap * (count - 1);
    let y = start + (available.saturating_sub(menu_height) / 2).min(2);
    let width = inner.width.min(30);
    for (index, action) in MenuAction::ALL.into_iter().enumerate() {
        let rect = Rect::new(
            inner.x + (inner.width - width) / 2,
            y + index as u16 * (height + gap),
            width,
            height,
        );
        let active = menu.selected == index;
        nav_button(
            frame,
            rect,
            action.key(),
            action.label(),
            active,
            p,
            p.panel,
        );
        menu.hitboxes.push((rect, index));
    }
    menu.back_rect = Rect::new(inner.x, inner.bottom() - 1, 14, 1);
    frame.render_widget(
        Paragraph::new("[Esc] Back").style(Style::new().fg(p.muted)),
        menu.back_rect,
    );
    let status_in_footer = y + menu_height >= inner.bottom() - 2;
    let hint = if status_in_footer && !menu.status.is_empty() {
        menu.status.as_str()
    } else {
        "[↑↓] Select · [Enter] Open"
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Right)
            .style(Style::new().fg(p.muted)),
        Rect::new(
            menu.back_rect.right(),
            menu.back_rect.y,
            inner.width - menu.back_rect.width,
            1,
        ),
    );
    if !status_in_footer && !menu.status.is_empty() {
        frame.render_widget(
            Paragraph::new(menu.status.as_str())
                .alignment(Alignment::Center)
                .style(Style::new().fg(p.muted)),
            Rect::new(inner.x, inner.bottom() - 2, inner.width, 1),
        );
    }
}
