use super::{
    Input,
    tests::{press, run, run_at},
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use rstop::{
    app::{App, UiMenu},
    ui,
};

fn draw(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    terminal
}

fn text(terminal: &Terminal<TestBackend>, rect: Rect) -> String {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn click(input: &mut Input<'_>, app: &mut App, x: u16, y: u16) {
    input.mouse(
        app,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
    );
    while let Some(key) = input.queued_key.take() {
        assert!(input.key(app, key));
    }
}

#[test]
fn device_info_wheel_and_end_reach_the_last_field_and_thumb_after_resize() {
    run(|input, app| {
        let mut extra: std::collections::BTreeMap<_, _> = (0..40)
            .map(|i| (format!("nvlink_field_{i:02}"), serde_json::json!(i)))
            .collect();
        extra.insert(
            "nvlink_zz_end_marker".into(),
            serde_json::json!("DEVICE_FIELDS_END"),
        );
        app.snapshot.gpus.push(rstop::model::Gpu {
            uuid: "scroll-gpu".into(),
            name: "Scroll GPU".into(),
            extra,
            ..Default::default()
        });
        app.set_view(rstop::app::View::Gpu);
        press(input, app, KeyCode::Char('I'));
        assert!(app.gpu_info_open());
        for (width, height) in [(60, 18), (160, 48), (80, 24), (220, 70)] {
            draw(app, width, height);
            press(input, app, KeyCode::Home);
            let terminal = draw(app, width, height);
            let area = app.table_rect;
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.y)].symbol(),
                "█"
            );
            for _ in 0..app.row_count() / 3 + 2 {
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column: area.right(),
                        row: area.bottom() - 1,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                draw(app, width, height);
            }
            let terminal = draw(app, width, height);
            assert!(
                text(
                    &terminal,
                    Rect::new(area.x, area.bottom() - 1, area.width, 1)
                )
                .contains("DEVICE_FIELDS_END")
            );
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.bottom() - 1)].symbol(),
                "█"
            );
            press(input, app, KeyCode::Home);
            draw(app, width, height);
            press(input, app, KeyCode::End);
            let terminal = draw(app, width, height);
            assert!(
                text(
                    &terminal,
                    Rect::new(area.x, area.bottom() - 1, area.width, 1)
                )
                .contains("DEVICE_FIELDS_END")
            );
            assert_eq!(
                terminal.backend().buffer()[(area.right(), area.bottom() - 1)].symbol(),
                "█"
            );
        }
    });
}

#[test]
fn long_menu_keeps_viewport_stable_and_pages_by_visible_rows() {
    run(|input, app| {
        app.ui_menu = Some(UiMenu::new(
            "All tools",
            (0..80)
                .map(|i| (format!("Tool {i:02}"), KeyCode::Null))
                .collect(),
            0,
        ));
        draw(app, 60, 18);
        let page = app.ui_menu.as_ref().unwrap().rect.height as usize;
        press(input, app, KeyCode::PageDown);
        draw(app, 60, 18);
        assert_eq!(app.ui_menu.as_ref().unwrap().offset, page);
        press(input, app, KeyCode::End);
        draw(app, 60, 18);
        let offset = app.ui_menu.as_ref().unwrap().offset;
        press(input, app, KeyCode::Up);
        draw(app, 60, 18);
        assert_eq!(app.ui_menu.as_ref().unwrap().offset, offset);
        press(input, app, KeyCode::BackTab);
        assert_eq!(app.ui_menu.as_ref().unwrap().selected, 77);
        press(input, app, KeyCode::Tab);
        assert_eq!(app.ui_menu.as_ref().unwrap().selected, 78);
        let back = app
            .popup_actions
            .iter()
            .find(|(_, k)| *k == KeyCode::Esc)
            .unwrap()
            .0;
        click(input, app, back.x, back.y);
        assert!(app.ui_menu.is_none());
        app.ui_menu = Some(UiMenu::new("Empty", vec![], 0));
        let terminal = draw(app, 60, 18);
        assert!(text(&terminal, app.popup_rect).contains("No items to display"));
        press(input, app, KeyCode::Enter);
        assert!(app.ui_menu.is_some());
    });
}

#[test]
fn settings_click_selects_then_arrows_adjust_and_save() {
    let path =
        std::env::temp_dir().join(format!("rstop-popup-settings-{}.toml", std::process::id()));
    run_at(&path, |input, app| {
        app.settings = true;
        draw(app, 60, 18);
        let row = app
            .dialog
            .settings_rows
            .iter()
            .find(|(_, i)| *i == 1)
            .unwrap()
            .0;
        click(input, app, row.x + 2, row.y);
        assert_eq!(app.setting_index, 1);
        assert_eq!(app.config.interval_ms, 1000);
        let right = app
            .dialog
            .settings_adjust
            .iter()
            .find(|(_, i, k)| *i == 1 && *k == KeyCode::Right)
            .unwrap()
            .0;
        click(input, app, right.x, right.y);
        assert_eq!(app.config.interval_ms, 1250);
        press(input, app, KeyCode::Enter);
        assert_eq!(app.config.interval_ms, 1500);
        press(input, app, KeyCode::Char('w'));
        assert!(path.is_file());
        assert!(app.dialog.settings_message.contains("Saved preferences"));
        press(input, app, KeyCode::End);
        let rendered = draw(app, 60, 18);
        assert_eq!(app.setting_index, rstop::config::Config::SETTINGS_COUNT - 1);
        assert!(text(&rendered, app.popup_rect).contains("32/32"));
        assert!(
            app.dialog
                .settings_rows
                .iter()
                .any(|(_, i)| *i == app.setting_index)
        );
    });
    let _ = std::fs::remove_file(path);
}

#[test]
fn invalid_forms_keep_target_and_value_and_mouse_cursor_is_unicode_safe() {
    run(|input, app| {
        let process = app.snapshot.processes[0].clone();
        app.prompt = Some(("nice".into(), "invalid".into(), Some(process)));
        press(input, app, KeyCode::Enter);
        assert!(!app.dialog.error.is_empty());
        let (_, value, target) = app.prompt.as_ref().unwrap();
        assert_eq!(value, "invalid");
        assert_eq!(target.as_ref().unwrap().pid, 42);
        press(input, app, KeyCode::Char('x'));
        assert!(app.dialog.error.is_empty());
        app.prompt.as_mut().unwrap().1 = "abc中文z".into();
        press(input, app, KeyCode::Home);
        draw(app, 60, 18);
        let field = app.dialog.input;
        click(input, app, field.x + 5, field.y);
        assert_eq!(app.editor.cursor, Some(6));
        press(input, app, KeyCode::Char('!'));
        assert_eq!(app.prompt.as_ref().unwrap().1, "abc中!文z");
        for kind in ["network target", "PCAP file", "folder path", "log file"] {
            app.prompt = Some((kind.into(), String::new(), None));
            press(input, app, KeyCode::Enter);
            assert!(app.prompt.is_some(), "{kind}");
            assert!(!app.dialog.error.is_empty(), "{kind}");
        }
    });
}

#[test]
fn capture_wizard_keeps_invalid_step_and_previous_values() {
    run(|input, app| {
        app.capture = Some(rstop::network_tools::Capture::default());
        app.prompt = Some(("capture interface".into(), String::new(), None));
        press(input, app, KeyCode::Enter);
        assert_eq!(app.prompt.as_ref().unwrap().0, "capture interface");
        assert!(!app.dialog.error.is_empty());
        app.prompt.as_mut().unwrap().1 = "test0".into();
        press(input, app, KeyCode::Enter);
        assert_eq!(app.capture.as_ref().unwrap().interface, "test0");
        assert_eq!(app.prompt.as_ref().unwrap().0, "capture filter");
        app.prompt = Some((
            "capture output".into(),
            input.path.to_string_lossy().into_owned(),
            None,
        ));
        // An empty output is rejected before any tcpdump task can start.
        app.prompt.as_mut().unwrap().1.clear();
        press(input, app, KeyCode::Enter);
        assert_eq!(app.prompt.as_ref().unwrap().0, "capture output");
        assert!(!app.dialog.error.is_empty());
        press(input, app, KeyCode::Esc);
        assert!(app.capture.is_none());
    });
}

#[test]
fn long_confirmation_scrolls_without_executing_and_n_cancels() {
    run(|input, app| {
        app.pending_plan = Some(rstop::management::Plan {
            title: "Review workload changes".into(),
            commands: (0..80)
                .map(|i| (format!("never-run-{i:02}"), vec![]))
                .collect(),
            changes_state: true,
        });
        app.prompt = Some(("workload action".into(), String::new(), None));
        draw(app, 60, 18);
        assert!(app.dialog.lines > app.dialog.rows);
        press(input, app, KeyCode::PageDown);
        assert!(app.dialog.scroll > 0);
        press(input, app, KeyCode::End);
        let terminal = draw(app, 60, 18);
        let rendered = text(&terminal, app.popup_rect);
        assert!(rendered.contains("never-run-79"), "{rendered}");
        assert!(rendered.contains("[y/Enter]"));
        press(input, app, KeyCode::Char('n'));
        assert!(app.pending_plan.is_none());
        assert!(app.prompt.is_none());
    });
}

#[test]
fn reports_and_help_have_bounded_scroll_and_separate_find_field() {
    run(|input, app| {
        app.detail = Some((
            "Report".into(),
            (0..100).map(|i| format!("Report row {i:02}\n")).collect(),
        ));
        draw(app, 60, 18);
        press(input, app, KeyCode::End);
        let terminal = draw(app, 60, 18);
        assert!(text(&terminal, app.popup_rect).contains("Report row 99"));
        let end = app.detail_scroll;
        press(input, app, KeyCode::Down);
        draw(app, 60, 18);
        assert_eq!(app.detail_scroll, end);
        press(input, app, KeyCode::Char('/'));
        for ch in "unmatched-token".chars() {
            press(input, app, KeyCode::Char(ch));
        }
        let terminal = draw(app, 60, 18);
        let rendered = text(&terminal, app.popup_rect);
        assert!(rendered.contains("No matching lines"));
        assert!(rendered.contains("Find: unmatched-token"));
        assert!(!app.dialog.input.is_empty());
        press(input, app, KeyCode::Esc);
        assert!(app.detail.is_some());
        assert!(!app.detail_search);
        app.detail = None;
        app.help = true;
        draw(app, 60, 18);
        press(input, app, KeyCode::End);
        draw(app, 60, 18);
        assert_eq!(app.detail_scroll, app.dialog.lines - app.detail_rows);
        assert!(app.detail_scroll > 0);
    });
}

#[test]
fn columns_toggle_in_place_and_trace_can_cancel_without_attaching() {
    run(|input, app| {
        press(input, app, KeyCode::Char('C'));
        draw(app, 60, 18);
        press(input, app, KeyCode::Down);
        let before = app.config.columns.clone();
        press(input, app, KeyCode::Char(' '));
        assert_ne!(app.config.columns, before);
        assert_eq!(app.ui_menu.as_ref().unwrap().selected, 1);
        press(input, app, KeyCode::Esc);
        app.move_selection(1);
        press(input, app, KeyCode::Char('x'));
        let terminal = draw(app, 60, 18);
        let rendered = text(&terminal, app.popup_rect);
        assert!(rendered.contains("Trace system calls"), "{rendered}");
        assert!(rendered.contains("[y/Enter]"));
        press(input, app, KeyCode::Char('n'));
        assert!(app.prompt.is_none());
        assert!(!app.detail_pending);
    });
}

#[test]
fn narrow_updates_keep_full_choices_and_installing_can_dismiss() {
    run(|input, app| {
        app.update_notice = Some(rstop::updates::Notice::new(rstop::updates::Release {
            tag_name: "v99.0.0".into(),
            ..Default::default()
        }));
        let terminal = draw(app, 60, 18);
        let rendered = text(&terminal, app.popup_rect);
        for label in ["Upgrade", "Ignore this session", "Skip this version"] {
            assert!(rendered.contains(label), "{rendered}");
        }
        press(input, app, KeyCode::Tab);
        assert_eq!(app.update_notice.as_ref().unwrap().selected, 1);
        press(input, app, KeyCode::Enter);
        assert!(app.update_notice.is_none());
        app.update_notice = Some(rstop::updates::Notice::new(
            rstop::updates::Release::default(),
        ));
        app.update_notice.as_mut().unwrap().phase = rstop::updates::Phase::Installing;
        draw(app, 60, 18);
        let close = app.update_buttons.iter().find(|(_, i)| *i == 1).unwrap().0;
        click(input, app, close.x, close.y);
        assert!(app.update_notice.is_none());
    });
}

#[test]
fn update_errors_remain_complete_and_scroll_independently_of_buttons() {
    run(|input, app| {
        app.update_notice = Some(rstop::updates::Notice::new(
            rstop::updates::Release::default(),
        ));
        app.update_notice.as_mut().unwrap().phase = rstop::updates::Phase::Failed(
            (0..30)
                .map(|i| format!("Failure detail line {i:02}\n"))
                .collect(),
        );
        draw(app, 60, 18);
        assert!(app.dialog.lines >= 30);
        assert!(app.dialog.rows < app.dialog.lines);
        press(input, app, KeyCode::PageDown);
        assert_eq!(app.update_notice.as_ref().unwrap().selected, 0);
        assert!(app.dialog.scroll > 0);
        for _ in 0..30 {
            press(input, app, KeyCode::PageDown);
        }
        let terminal = draw(app, 60, 18);
        assert!(text(&terminal, app.popup_rect).contains("Failure detail line 29"));
        let end = app.dialog.scroll;
        input.mouse(
            app,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: app.dialog.content.x,
                row: app.dialog.content.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(app.dialog.scroll, end - 1);
        press(input, app, KeyCode::Esc);
        assert!(app.update_notice.is_none());
    });
}
