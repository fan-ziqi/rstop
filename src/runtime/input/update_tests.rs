use super::tests::{press, run};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::menu::MenuAction,
    updates::{Notice, Phase, Release},
};

#[test]
fn escape_menu_opens_update_window_by_click_enter_and_shortcut() {
    for activation in ["click", "enter", "shortcut"] {
        for (width, height) in [(60, 18), (80, 24), (160, 48)] {
            run(|input, app| {
                app.config.check_updates = false;
                app.config.ignored_update_version = Some("99.0.0".into());
                input.read_only = true;
                let selection = app.selected_process;
                press(input, app, KeyCode::Esc);
                assert!(app.main_menu.is_some());
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                let index = MenuAction::ALL
                    .iter()
                    .position(|action| *action == MenuAction::CheckUpdates)
                    .unwrap();
                match activation {
                    "click" => {
                        let rect = app.main_menu.as_ref().unwrap().hitboxes[index].0;
                        input.mouse(
                            app,
                            MouseEvent {
                                kind: MouseEventKind::Down(MouseButton::Left),
                                column: rect.right() - 1,
                                row: rect.bottom() - 1,
                                modifiers: KeyModifiers::NONE,
                            },
                        );
                        let key = input.queued_key.take().unwrap();
                        assert!(input.key(app, key));
                    }
                    "enter" => {
                        for _ in 0..index {
                            press(input, app, KeyCode::Down);
                        }
                        press(input, app, KeyCode::Enter);
                    }
                    _ => {
                        press(input, app, KeyCode::Char('u'));
                    }
                }
                assert!(input.update_check_requested);
                assert!(matches!(
                    app.update_notice.as_ref().unwrap().phase,
                    Phase::Checking
                ));
                terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
                assert_eq!(app.update_buttons.len(), 1); // No release link before a result.
                press(input, app, KeyCode::Char('u'));
                press(input, app, KeyCode::Char('n'));
                assert!(matches!(
                    app.update_notice.as_ref().unwrap().phase,
                    Phase::Checking
                ));
                press(input, app, KeyCode::Esc);
                assert!(app.update_notice.is_none());
                assert!(app.main_menu.is_some());
                assert_eq!(app.selected_process, selection);
                assert_eq!(app.config.ignored_update_version.as_deref(), Some("99.0.0"));
            });
        }
    }
}

#[test]
fn failed_manual_check_retries_checking_and_current_result_closes_to_menu() {
    run(|input, app| {
        press(input, app, KeyCode::Esc);
        press(input, app, KeyCode::Char('u'));
        input.update_check_requested = false;
        app.update_notice
            .as_mut()
            .unwrap()
            .finish_check(Err("Offline".into()));
        let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
        terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
        press(input, app, KeyCode::Enter);
        assert!(input.update_check_requested);
        assert!(input.updating_release.is_none());
        let notice = app.update_notice.as_mut().unwrap();
        assert!(matches!(notice.phase, Phase::Checking));
        notice.finish_check(Ok(Release {
            tag_name: format!("v{}", env!("CARGO_PKG_VERSION")),
            html_url: format!(
                "https://github.com/fan-ziqi/rstop/releases/tag/v{}",
                env!("CARGO_PKG_VERSION")
            ),
            ..Default::default()
        }));
        assert!(matches!(notice.phase, Phase::UpToDate));
        terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
        press(input, app, KeyCode::Char('u'));
        assert!(input.updating_release.is_none());
        let rect = app
            .update_buttons
            .iter()
            .find(|(_, index)| *index == 1)
            .unwrap()
            .0;
        input.mouse(
            app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: rect.x,
                row: rect.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        let key = input.queued_key.take().unwrap();
        assert!(input.key(app, key));
        assert!(app.update_notice.is_none());
        assert!(app.main_menu.is_some());
    });
}

#[test]
fn reopening_update_window_keeps_an_active_install_in_progress() {
    run(|input, app| {
        input.updating_release = Some(Release {
            tag_name: "v99.0.0".into(),
            ..Default::default()
        });
        press(input, app, KeyCode::Esc);
        press(input, app, KeyCode::Char('u'));
        assert!(!input.update_check_requested);
        assert!(matches!(
            app.update_notice.as_ref().unwrap().phase,
            Phase::Installing
        ));
        press(input, app, KeyCode::Esc);
        assert!(app.update_notice.is_none());
        assert!(input.updating_release.is_some());
        press(input, app, KeyCode::Char('u'));
        assert!(matches!(
            app.update_notice.as_ref().unwrap().phase,
            Phase::Installing
        ));
    });
}

#[test]
fn manual_check_can_reveal_skipped_release_without_allowing_read_only_install() {
    run(|input, app| {
        input.read_only = true;
        app.config.ignored_update_version = Some("99.0.0".into());
        app.update_notice = Some(Notice::checking());
        app.update_notice
            .as_mut()
            .unwrap()
            .finish_check(Ok(Release {
                tag_name: "v99.0.0".into(),
                html_url: "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0".into(),
                ..Default::default()
            }));
        assert!(matches!(
            app.update_notice.as_ref().unwrap().phase,
            Phase::Available
        ));
        let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
        terminal.draw(|frame| rstop::ui::draw(frame, app)).unwrap();
        press(input, app, KeyCode::Char('u'));
        assert!(matches!(
            app.update_notice.as_ref().unwrap().phase,
            Phase::Failed(_)
        ));
        assert!(input.updating_release.is_none());
    });
}
