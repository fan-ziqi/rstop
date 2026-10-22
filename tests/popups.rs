use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use rstop::{
    app::{App, SignalTarget, UiMenu, View},
    config::Config,
    model::{Process, Snapshot},
    ui,
};
#[path = "../src/export.rs"]
mod export;

fn app(theme: &str) -> App {
    let mut app = App::new(Config {
        theme: theme.into(),
        ..Default::default()
    });
    app.ingest(Snapshot {
        processes: vec![Process {
            pid: 4242,
            start_time: 100,
            name: "python train.py".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    app.set_view(View::Processes);
    app.move_selection(1);
    app
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

fn preview(app: &mut App, name: &str, width: u16, height: u16) {
    if let Ok(directory) = std::env::var("RSTOP_POPUP_PREVIEW_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        export::svg(
            &directory.join(format!("{name}-{width}x{height}-{}.svg", app.config.theme)),
            width,
            height,
            app,
        )
        .unwrap();
    }
}

#[test]
fn compact_dialogs_show_real_targets_and_clickable_actions_across_sizes_and_themes() {
    for theme in ["black", "white", "graphite"] {
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            let mut app = app(theme);
            let target = SignalTarget {
                pid: 4242,
                start_time: 100,
                name: "python train.py".into(),
                signal: sysinfo::Signal::Kill,
            };
            app.signal_targets.push(target.clone());
            app.signal = Some(target);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            let rect = app.popup_rect;
            assert!(rect.width <= 84 && rect.height <= height - 2);
            if height >= 30 {
                assert_eq!(app.signal_ui.hits.len(), sysinfo::SUPPORTED_SIGNALS.len());
            }
            let rendered = text(&terminal, rect);
            for expected in [
                "Kill process?",
                "SIGKILL",
                "4242",
                "python train.py",
                "[k]",
                "Confirm",
                "Cancel",
                "[y/Enter]",
                "[n/Esc]",
                "available",
            ] {
                assert!(
                    rendered.contains(expected),
                    "{width}x{height} {theme}: {expected}: {rendered}"
                );
            }
            for (bounds, _) in &app.popup_actions {
                assert!(
                    rect.contains((bounds.x, bounds.y).into())
                        && rect.contains((bounds.right() - 1, bounds.bottom() - 1).into())
                );
            }
            assert!(
                app.popup_actions
                    .iter()
                    .any(|(_, key)| *key == KeyCode::Char('y'))
            );
            assert!(
                app.popup_actions
                    .iter()
                    .any(|(_, key)| *key == KeyCode::Char('n'))
            );
            preview(&mut app, "signal", width, height);
            app.signal = None;
            app.prompt = Some((
                "nice".into(),
                "10".into(),
                Some(app.snapshot.processes[0].clone()),
            ));
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            let rendered = text(&terminal, app.popup_rect);
            for expected in ["Process priority", "4242", "10", "Apply", "Cancel"] {
                assert!(rendered.contains(expected), "{rendered}");
            }
            assert_eq!(app.popup_rect.height, 14);
            preview(&mut app, "prompt", width, height);
        }
    }
}

#[test]
fn menus_reports_settings_and_update_share_padded_surfaces_without_oversized_empty_boxes() {
    for (width, height) in [(60, 18), (100, 30), (160, 48)] {
        let mut app = app("black");
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        app.ui_menu = Some(UiMenu::new("Process tools", app.tools_commands(), 0));
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        let rect = app.popup_rect;
        let menu = app.ui_menu.as_ref().unwrap();
        assert!(menu.rect.x >= rect.x + 3 && menu.rect.y >= rect.y + 2);
        assert!(menu.rect.bottom() <= rect.bottom() - 3);
        preview(&mut app, "tools", width, height);
        app.ui_menu = None;
        app.detail = Some(("Process status".into(), "PID 4242\nRunning normally".into()));
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        assert_eq!(app.popup_rect.height, 9);
        assert!(app.popup_rect.width <= 56);
        assert!(text(&terminal, app.popup_rect).contains("Running normally"));
        preview(&mut app, "detail", width, height);
        app.detail = None;
        app.settings = true;
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        let rendered = text(&terminal, app.popup_rect);
        for expected in ["Settings", "Sampling interval", "Save preferences"] {
            assert!(rendered.contains(expected), "{rendered}");
        }
        preview(&mut app, "settings", width, height);
        app.settings = false;
        app.update_notice = Some(rstop::updates::Notice::new(rstop::updates::Release {
            tag_name: "v99.0.0".into(),
            html_url: "https://github.com/fan-ziqi/rstop/releases/tag/v99.0.0".into(),
            ..Default::default()
        }));
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        assert!(!app.update_buttons.is_empty());
        preview(&mut app, "update", width, height);
    }
}

#[test]
fn remaining_popups_fit_minimum_terminal_and_plain_terminal_modes() {
    for theme in ["black", "white", "graphite"] {
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            let mut app = app(theme);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            app.main_menu = Some(Default::default());
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            preview(&mut app, "main", width, height);
            app.main_menu = None;
            app.help = true;
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            preview(&mut app, "help", width, height);
            app.help = false;
            app.detail = Some(("Process status".into(), "PID 4242\nRunning normally".into()));
            app.detail_search = true;
            app.detail_filter = "missing".into();
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            assert!(text(&terminal, app.popup_rect).contains("No matching lines"));
            preview(&mut app, "search", width, height);
            app.detail = None;
            app.detail_search = false;
            app.detail_filter.clear();
            app.prompt = Some((
                "nice".into(),
                "invalid".into(),
                Some(app.snapshot.processes[0].clone()),
            ));
            app.dialog.error = "Enter a number between -20 and 19".into();
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            assert!(text(&terminal, app.popup_rect).contains("Enter a number"));
            preview(&mut app, "invalid-input", width, height);
            app.dialog.error.clear();
            app.prompt = Some((
                "trace".into(),
                String::new(),
                Some(app.snapshot.processes[0].clone()),
            ));
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            preview(&mut app, "trace", width, height);
            app.prompt = None;
            app.snapshot.gpus.push(rstop::model::Gpu {
                id: 0,
                name: "NVIDIA Test GPU".into(),
                vendor: "NVIDIA".into(),
                driver: "595.1".into(),
                ..Default::default()
            });
            app.set_view(View::Gpu);
            app.focus_panel(2);
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            let rendered = text(&terminal, app.popup_rect);
            for label in ["DEVICE INFO", "Filter:", "fields", "Back", "Inspect"] {
                assert!(rendered.contains(label), "{rendered}");
            }
            preview(&mut app, "device-info", width, height);
            app.focus_panel(0);
            app.set_view(View::Processes);
            app.settings = true;
            app.setting_index = rstop::config::Config::SETTINGS_COUNT - 1;
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            preview(&mut app, "settings-last", width, height);
            app.settings = false;
            for ascii in [false, true] {
                app.config.ascii = ascii;
                app.config.no_color = true;
                app.help = true;
                terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
                let rendered = text(&terminal, app.popup_rect);
                assert!(rendered.contains("Keyboard shortcuts"));
                assert!(rendered.contains("Back"));
                if ascii {
                    assert!(!rendered.contains('←'));
                }
            }
        }
    }
}
