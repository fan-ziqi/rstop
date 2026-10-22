use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, UiMenu, View, editor::Editor},
    config::Config,
    model::Snapshot,
    plot::{TimeScale, Window},
    ui,
};
use std::collections::VecDeque;

#[test]
fn utf8_editing_and_restored_text_keep_a_valid_cursor() {
    let mut editor = Editor::default();
    let mut text = "甲乙 ab".to_string();
    for code in [
        KeyCode::Home,
        KeyCode::Right,
        KeyCode::Delete,
        KeyCode::Char('丙'),
        KeyCode::End,
    ] {
        assert!(editor.edit(&mut text, KeyEvent::new(code, KeyModifiers::NONE)));
    }
    assert_eq!(text, "甲丙 ab");
    editor.edit(
        &mut text,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
    );
    assert_eq!(text, "甲丙 ");
    assert_eq!(editor.display("新"), "新▏");
    editor.edit(
        &mut text,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert!(text.is_empty());
}

fn timed(
    values: &[f64],
    times: &[f64],
    periods: &[f64],
    seconds: u64,
    width: usize,
) -> Vec<Option<rstop::plot::Range>> {
    let times: VecDeque<_> = times.iter().copied().collect();
    let periods: VecDeque<_> = periods.iter().copied().collect();
    Window {
        samples: values.len(),
        end: values.len() as u64,
        time: Some(TimeScale {
            timestamps: &times,
            periods: &periods,
            seconds,
        }),
    }
    .project(&values.iter().copied().collect(), width)
}

#[test]
fn elapsed_time_zoom_preserves_geometry_rates_peaks_and_gaps() {
    let before = timed(&[0., 20., 40.], &[0., 1., 2.], &[1.; 3], 10, 20);
    let after = timed(&[0., 20., 40., 60.], &[0., 1., 2., 3.], &[1.; 4], 10, 20);
    for x in 15..20 {
        assert_eq!(before[x].map(|r| r.last), after[x - 2].map(|r| r.last));
    }
    assert!(before[..15].iter().all(Option::is_none));
    let variable = timed(
        &[0., 10., 20., 30., 40.],
        &[0., 1., 1.5, 2., 3.],
        &[1., 1., 0.5, 0.5, 1.],
        4,
        8,
    );
    assert_eq!(variable[3].unwrap().last, 10.);
    assert_eq!(variable[4].unwrap().last, 20.);
    let gap = timed(&[0., 10., 80., 90.], &[0., 1., 8., 9.], &[1.; 4], 10, 10);
    assert!(gap[2..8].iter().all(Option::is_none));
    let compressed = timed(
        &[10., f64::NAN, 90., 30.],
        &[0., 0.1, 0.2, 0.3],
        &[0.1; 4],
        1,
        1,
    );
    assert_eq!(compressed[0].unwrap().high, 90.);
    assert!(compressed[0].unwrap().gap);
}

#[test]
fn editors_do_not_move_tables_and_dialogs_dim_only_the_background() {
    let mut app = App::new(Config::default());
    app.ingest(Snapshot::default());
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    app.set_view(View::Processes);
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    let rect = app.table_rect;
    let input = app.filter_rect;
    app.begin_filter();
    app.filter = "name:worker".into();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_eq!(app.table_rect, rect);
    assert_eq!(app.filter_rect, input);
    app.editing_filter = false;
    for &theme in rstop::config::THEMES {
        app.config.theme = theme.into();
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        let p = ui::palette(theme);
        let buffer = terminal.backend().buffer();
        for (rect, _) in &app.tab_ranges {
            for x in rect.left()..rect.right() {
                assert_eq!(buffer[(x, rect.y + 1)].bg, p.bg);
            }
        }
        let mut previous_end = app.filter_rect.right();
        for (rect, _) in &app.controls {
            let background = if rect.y == app.controls_rect.y {
                assert!(rect.x >= previous_end);
                previous_end = rect.right();
                p.panel
            } else {
                p.bg
            };
            for x in rect.left()..rect.right() {
                assert_eq!(buffer[(x, rect.y)].bg, background);
            }
        }
    }
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    let foreground = terminal.backend().buffer()[(1, 0)].fg;
    app.ui_menu = Some(UiMenu::new("Commands", app.commands(), 0));
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_ne!(terminal.backend().buffer()[(1, 0)].fg, foreground);
    for (rect, _) in &app.tab_ranges {
        // No detached gray fill behind labels or padding.
        assert_eq!(
            terminal.backend().buffer()[(rect.x + 1, rect.y + 1)].bg,
            terminal.backend().buffer()[(rect.x + 2, rect.y + 1)].bg
        );
    }
}

#[test]
fn workload_keys_are_unique_and_visible_windows_respect_retention() {
    for actions in [
        rstop::management::service_actions(),
        rstop::management::container_actions(),
    ] {
        let mut keys = std::collections::HashSet::new();
        for action in actions {
            assert!(
                keys.insert(
                    rstop::management::action_key(action).expect("visible action has a key")
                )
            );
        }
    }
    let mut app = App::new(Config::default());
    app.set_time_window(900);
    assert_eq!(app.config.history, 900);
    app.config.interval_ms = 250;
    app.set_time_window(3600);
    assert_eq!(app.config.time_window_seconds, 900);
    assert_eq!(app.config.history, 3600);
}

#[test]
fn main_menu_and_project_links_fit_small_terminals_and_accessible_modes() {
    use ratatui::style::Color;
    use rstop::app::menu::{MainMenu, MenuAction};
    for (width, height) in [(160, 48), (80, 24), (60, 18), (60, 29), (60, 20), (80, 28)] {
        for &theme in rstop::config::THEMES {
            for (ascii, no_color) in [(false, false), (true, false), (false, true), (true, true)] {
                let mut app = App::new(Config {
                    theme: theme.into(),
                    ascii,
                    no_color,
                    ..Default::default()
                });
                app.main_menu = Some(MainMenu::default());
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
                let buffer = terminal.backend().buffer();
                let text = buffer
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                let menu = app.main_menu.as_ref().unwrap();
                assert_eq!(menu.hitboxes.len(), 3);
                assert_eq!(menu.links.len(), 3);
                for pair in menu.hitboxes.windows(2) {
                    assert_eq!(pair[0].0.x, pair[1].0.x);
                    assert!(pair[0].0.bottom() <= pair[1].0.y);
                }
                for (rect, _) in &menu.hitboxes {
                    assert!(rect.width > 0 && rect.height > 0);
                    assert!(rect.bottom() <= menu.back_rect.y);
                    for (link, _) in &menu.links {
                        assert!(link.bottom() <= rect.y);
                    }
                    for row in rect.top()..rect.bottom() {
                        for column in rect.left()..rect.right() {
                            assert_eq!(
                                buffer[(column, row)].bg,
                                if no_color {
                                    Color::Reset
                                } else {
                                    ui::palette(theme).panel
                                }
                            );
                        }
                    }
                }
                for (index, action) in MenuAction::ALL.into_iter().enumerate() {
                    let label = if menu.hitboxes[index].0.height == 1 {
                        action.label().to_string()
                    } else {
                        rstop::app::commands::key_name(action.key())
                    };
                    assert!(text.contains(&label), "{width}x{height}: {label}");
                }
                for value in [
                    "Ziqi Fan",
                    "Apache-2.0",
                    rstop::project::SOURCE,
                    rstop::project::COFFEE,
                ] {
                    assert!(
                        text.contains(
                            value
                                .trim_start_matches("https://")
                                .trim_start_matches("www.")
                        )
                    );
                }
                assert!(!text.contains("范子琦"));
                if ascii {
                    assert!(text.is_ascii());
                }
                if no_color {
                    assert!(
                        buffer
                            .content
                            .iter()
                            .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
                    );
                }
            }
        }
    }
}

#[test]
fn menu_is_static_and_refreshes_its_backdrop_after_theme_or_size_changes() {
    let mut app = App::new(Config::default());
    app.paused = true;
    app.main_menu = Some(Default::default());
    let mut terminal = Terminal::new(TestBackend::new(140, 44)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    let first = terminal.backend().buffer().clone();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_eq!(&first, terminal.backend().buffer());
    assert!(app.paused);
    app.config.theme = "nord".into();
    terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_eq!(app.main_menu.as_ref().unwrap().backdrop_theme, "nord");
    let mut small = Terminal::new(TestBackend::new(60, 18)).unwrap();
    small.draw(|frame| ui::draw(frame, &mut app)).unwrap();
    assert_eq!(
        app.main_menu
            .as_ref()
            .unwrap()
            .backdrop
            .as_ref()
            .unwrap()
            .area
            .width,
        60
    );
    assert!(
        app.main_menu
            .as_ref()
            .unwrap()
            .hitboxes
            .iter()
            .all(|(rect, _)| rect.bottom() < 18)
    );
}

#[test]
fn common_action_bar_keeps_keycaps_and_primary_actions_visible_at_every_supported_size() {
    use rstop::model::Gpu;
    for (width, height) in [(60, 18), (80, 24), (160, 48)] {
        let mut app = App::new(Config::default());
        app.snapshot.gpus.push(Gpu::default());
        // Long feedback belongs to the status strip and cannot displace actions.
        app.status = "A long resource operation result ".repeat(8);
        for view in [
            View::Overview,
            View::Processes,
            View::Gpu,
            View::Storage,
            View::Network,
            View::System,
            View::Containers,
            View::Services,
        ] {
            app.set_view(view);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let footer: String = (0..width)
                .map(|x| buffer[(x, height - 1)].symbol())
                .collect();
            for label in ["[Esc]", "[F6] Tools", "[F10] Quit"] {
                assert!(
                    footer.contains(label),
                    "{width}x{height} {view:?}: {footer}"
                );
            }
            if app.can_filter() {
                assert!(footer.contains("[F3] Filter"));
            }
            if app.has_process_table()
                || matches!(view, View::Network | View::Containers | View::Services)
            {
                assert!(footer.contains("[F4] Sort"));
            }
            if width >= 80 {
                assert!(footer.contains("[F5] View"));
            }
            if width >= 160 {
                assert!(footer.contains("[Enter] Details"));
            }
            let mut right = 0;
            for (rect, key) in &app.footer_actions {
                assert!(rect.x >= right && rect.right() <= width);
                right = rect.right();
                let expected = format!("[{}]", rstop::app::commands::key_name(*key));
                let actual: String = (rect.x..rect.right())
                    .map(|x| buffer[(x, rect.y)].symbol())
                    .collect();
                assert!(actual.contains(&expected));
                assert!(
                    buffer[(rect.x + 1, rect.y)]
                        .modifier
                        .contains(ratatui::style::Modifier::BOLD)
                );
            }
            if view == View::Network {
                assert_eq!(app.panel_regions.len(), 2);
                let text: String = buffer.content.iter().map(|c| c.symbol()).collect();
                assert!(!text.contains("N test"));
                assert!(!text.contains("m tools"));
            }
        }
    }
}
