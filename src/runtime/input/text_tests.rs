use super::tests::run;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};
use rstop::{app::App, ui};

fn draw(app: &mut App) {
    Terminal::new(TestBackend::new(160, 48))
        .unwrap()
        .draw(|frame| ui::draw(frame, app))
        .unwrap();
}

#[test]
fn dragging_copies_and_shortcuts_keep_pane_open_until_selection_is_cleared() {
    run(|input, app| {
        app.move_selection(1);
        assert!(app.open_selection());
        draw(app);
        let area = app.text_regions[0].buffer.area;
        let mut mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        };
        input.mouse(app, mouse);
        mouse.kind = MouseEventKind::Drag(MouseButton::Left);
        mouse.column += 5;
        input.mouse(app, mouse);
        draw(app);
        mouse.kind = MouseEventKind::Up(MouseButton::Left);
        input.mouse(app, mouse);
        assert!(app.clipboard_request.is_none());
        for (code, modifiers) in [
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
            (
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            (
                KeyCode::Char('C'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
        ] {
            assert!(input.key(app, KeyEvent::new(code, modifiers)));
            assert_eq!(app.clipboard_request.take().as_deref(), Some("worker"));
            assert!(app.selection_open);
        }
        assert!(input.key(app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(app.selection_open);
        assert!(app.text_selection.selected_text().is_none());
        assert!(input.key(
            app,
            KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )
        ));
        assert!(!input.key(
            app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ));
    });
}

#[test]
fn report_text_does_not_select_background_or_include_search_and_buttons() {
    run(|input, app| {
        app.move_selection(1);
        app.open_selection();
        app.detail = Some(("Report".into(), "alpha中文\nbeta".into()));
        draw(app);
        let reports: Vec<_> = app
            .text_regions
            .iter()
            .filter(|region| !region.is_screen())
            .collect();
        assert_eq!(reports.len(), 1);
        let region = reports[0].clone();
        assert!(region.id.starts_with("report:"));
        let area = region.buffer.area;
        app.text_selection.begin(region, (area.x, area.y).into());
        input.mouse(
            app,
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: area.x + 3,
                row: area.y + 1,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(app.clipboard_request.is_none());
        assert!(input.key(
            app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ));
        assert_eq!(app.clipboard_request.as_deref(), Some("alpha中文\nbeta"));
        app.ui_menu = Some(rstop::app::UiMenu::new("Tools", vec![], 0));
        draw(app);
        assert!(app.text_regions.iter().all(|region| region.is_screen()));
        assert!(app.text_selection.selected_text().is_none());
    });
}

#[test]
fn sidebar_wheel_and_alt_end_reach_the_final_field_after_resize() {
    run(|input, app| {
        app.snapshot.processes[0].command = "long argument 中文 ".repeat(100);
        app.snapshot.processes[0].write_total = 987654321;
        app.remember_selection();
        assert!(app.open_selection());
        let identity = app.selected_process;
        for (width, height) in [(160, 48), (60, 18), (120, 30), (220, 70)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| ui::draw(frame, app)).unwrap();
            input.key(app, KeyEvent::new(KeyCode::Home, KeyModifiers::ALT));
            terminal.draw(|frame| ui::draw(frame, app)).unwrap();
            let area = app.text_regions[0].buffer.area;
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.y)].symbol(),
                "█"
            );
            let wheel = MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: area.right(),
                row: area.bottom() - 1,
                modifiers: KeyModifiers::NONE,
            };
            for _ in 0..app.selection_lines / 3 + 2 {
                input.mouse(app, wheel);
                terminal.draw(|frame| ui::draw(frame, app)).unwrap();
            }
            let bottom = app.selection_scroll;
            let last_row = (area.x..area.right())
                .map(|x| terminal.backend().buffer()[(x, area.bottom() - 1)].symbol())
                .collect::<String>();
            assert!(
                last_row.contains("987654321"),
                "{width}x{height}: {last_row}"
            );
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.bottom() - 1)].symbol(),
                "█"
            );
            assert_eq!(app.selected_process, identity);
            input.key(app, KeyEvent::new(KeyCode::Home, KeyModifiers::ALT));
            terminal.draw(|frame| ui::draw(frame, app)).unwrap();
            input.key(app, KeyEvent::new(KeyCode::End, KeyModifiers::ALT));
            terminal.draw(|frame| ui::draw(frame, app)).unwrap();
            assert_eq!(app.selection_scroll, bottom);
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.bottom() - 1)].symbol(),
                "█"
            );
        }
    });
}
