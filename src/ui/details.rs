use super::{Palette, block, charts::chart};
use crate::app::App;
use ratatui::{prelude::*, widgets::*};
pub(super) fn explorer(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let rows = app.extra_rows();
    let data = rows.iter().map(|(name, value)| {
        Row::new([
            name.clone(),
            match value {
                serde_json::Value::Array(v) => format!("{} records · Enter inspect", v.len()),
                serde_json::Value::Object(v) if v.contains_key("error") => {
                    format!("{}", v["error"])
                }
                serde_json::Value::Object(v) => format!("{} fields · Enter inspect", v.len()),
                serde_json::Value::String(v) => v
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(120)
                    .collect(),
                _ => value.to_string().chars().take(120).collect(),
            },
        ])
    });
    let b = super::controls::selectable_block(" SYSTEM EXPLORER ".into(), area, app, p, p.blue);
    let inner = b.inner(area);
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let mut state = TableState::default()
        .with_selected(if rows.is_empty() {
            None
        } else {
            Some(app.selected)
        })
        .with_offset(app.table_offset);
    frame.render_stateful_widget(
        Table::new(
            data,
            [Constraint::Percentage(35), Constraint::Percentage(65)],
        )
        .header(Row::new(["SOURCE / CAPABILITY", "DATA / STATUS"]))
        .block(b)
        .row_highlight_style(Style::new().bg(p.selection)),
        area,
        &mut state,
    );
    app.table_offset = state.offset();
}
pub(super) fn history_view(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let cols =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(area);
    let b = super::controls::selectable_block(" SERIES ".into(), cols[0], app, p, p.mint);
    let keys: Vec<_> = app
        .history
        .series
        .keys()
        .filter(|key| key.to_lowercase().contains(&app.filter.to_lowercase()))
        .collect();
    let mut state = ListState::default()
        .with_offset(app.table_offset)
        .with_selected(Some(app.selected.min(keys.len().saturating_sub(1))));
    app.table_rect = Rect::new(
        cols[0].x + 1,
        cols[0].y + 1,
        cols[0].width.saturating_sub(2),
        cols[0].height.saturating_sub(2),
    );
    frame.render_stateful_widget(
        List::new(keys.iter().map(|s| ListItem::new(s.as_str())))
            .block(b)
            .highlight_style(Style::new().bg(p.selection)),
        cols[0],
        &mut state,
    );
    app.table_offset = state.offset();
    if let Some(key) = keys.get(app.selected) {
        let history = &app.history.series[*key];
        let max = if key.contains("(%)") && !key.contains("main CPU") {
            100.0
        } else {
            crate::plot::nice_max(history.iter().copied().fold(1.0, f64::max) * 1.1)
        };
        let b = block(
            format!(
                " {key} · {} samples · latest {:.2} ",
                history.len(),
                history.back().unwrap_or(&0.0)
            ),
            p,
            p.blue,
        );
        let inner = b.inner(cols[1]);
        frame.render_widget(b, cols[1]);
        chart(
            frame,
            inner,
            "sample history",
            &[("value", history, p.mint)],
            max,
            key.rsplit_once(" (")
                .map(|(_, unit)| unit.trim_end_matches(')'))
                .unwrap_or(""),
            app.plot_window(),
            p,
        );
    }
}
pub(super) fn event_view(frame: &mut Frame, area: Rect, app: &mut App, p: Palette) {
    let b = super::controls::selectable_block(
        format!(" EVENTS · {}s hold ", app.config.alert_hold_seconds),
        area,
        app,
        p,
        p.amber,
    );
    let inner = b.inner(area);
    app.table_rect = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let count = app.row_count();
    let mut state = TableState::default()
        .with_selected((count > 0).then_some(app.selected))
        .with_offset(app.table_offset);
    let rows = app
        .events
        .history
        .iter()
        .rev()
        .filter(|event| crate::filter::event_matches(event, &app.filter))
        .map(|e| {
            Row::new([
                e.timestamp.to_string(),
                if e.active { "ACTIVE" } else { "RECOVERED" }.into(),
                e.condition.clone(),
            ])
            .style(Style::new().fg(if e.active { p.amber } else { p.muted }))
        });
    frame.render_stateful_widget(
        Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Min(15),
            ],
        )
        .header(Row::new(["UNIX TIME", "STATE", "CONDITION"]))
        .block(b)
        .row_highlight_style(Style::new().bg(p.selection))
        .highlight_symbol("▎"),
        area,
        &mut state,
    );
    app.table_offset = state.offset();
}
