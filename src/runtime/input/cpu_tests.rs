use super::{
    Input,
    tests::{press, run, run_at},
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use rstop::{
    app::{App, View},
    config::Config,
    model::{CoreTime, Snapshot},
    ui,
};

fn draw(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
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
fn dispatch(input: &mut Input<'_>, app: &mut App) {
    while let Some(key) = input.queued_key.take() {
        assert!(input.key(app, key));
    }
}
fn click(input: &mut Input<'_>, app: &mut App, rect: Rect) {
    input.mouse(
        app,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.right() - 1,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        },
    );
    dispatch(input, app);
}
fn supported(app: &mut App) {
    app.set_view(View::Overview);
    app.config.display.overview_panel = "cpu".into();
    for elapsed in 0..3 {
        app.ingest(Snapshot {
            elapsed: elapsed as f64,
            cores: vec![70.; 7],
            core_times: vec![
                CoreTime {
                    nice: 10.,
                    user: 30.,
                    system: 20.,
                    steal: 10.,
                    iowait: 20.
                };
                7
            ],
            ..Default::default()
        });
    }
}

#[test]
fn cpu_header_mode_button_uses_its_drawn_rect_and_flips_real_mouse_rendering() {
    run(|input, app| {
        supported(app);
        for (width, height) in [(80, 40), (160, 48), (300, 120), (100, 30)] {
            app.config.display.cpu_cores = "bars".into();
            let terminal = draw(app, width, height);
            let rect = app
                .controls
                .iter()
                .find(|(_, key)| *key == KeyCode::Char('b'))
                .unwrap()
                .0;
            assert!(rect.right() <= width && rect.bottom() <= height);
            assert!(text(&terminal, rect).contains("[b] History"));
            assert!(text(&terminal, terminal.backend().buffer().area).contains("Per-core usage"));
            click(input, app, rect);
            assert_eq!(app.config.display.cpu_cores, "history");
            let terminal = draw(app, width, height);
            let rect = app
                .controls
                .iter()
                .find(|(_, key)| *key == KeyCode::Char('b'))
                .unwrap()
                .0;
            assert!(text(&terminal, rect).contains("[b] Bars"));
            let rendered = text(&terminal, terminal.backend().buffer().area);
            assert!(rendered.contains("Per-core history"));
            assert!(!rendered.contains('▕'));
            click(input, app, rect);
            assert_eq!(app.config.display.cpu_cores, "bars");
            let terminal = draw(app, width, height);
            assert!(text(&terminal, terminal.backend().buffer().area).contains('▕'));
        }
    });
}

#[test]
fn cpu_view_menu_dispatches_the_same_toggle_and_only_w_persists_it() {
    let path = std::env::temp_dir().join(format!(
        "rstop-cpu-mode-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    run_at(&path, |input, app| {
        supported(app);
        press(input, app, KeyCode::F(5));
        let menu = app.ui_menu.as_mut().unwrap();
        menu.selected = menu
            .items
            .iter()
            .position(|(label, code)| {
                label == "CPU cores: switch to history" && *code == KeyCode::Char('b')
            })
            .unwrap();
        press(input, app, KeyCode::Enter);
        dispatch(input, app);
        assert_eq!(app.config.display.cpu_cores, "history");
        assert!(!path.exists());
        let terminal = draw(app, 160, 48);
        assert!(text(&terminal, terminal.backend().buffer().area).contains("Per-core history"));
        press(input, app, KeyCode::Char('w'));
        assert_eq!(Config::load(&path).unwrap().display.cpu_cores, "history");
        press(input, app, KeyCode::F(5));
        let menu = app.ui_menu.as_mut().unwrap();
        menu.selected = menu
            .items
            .iter()
            .position(|(label, _)| label == "CPU cores: switch to bars")
            .unwrap();
        press(input, app, KeyCode::Enter);
        dispatch(input, app);
        assert_eq!(app.config.display.cpu_cores, "bars");
        assert_eq!(Config::load(&path).unwrap().display.cpu_cores, "history");
    });
    std::fs::remove_file(path).unwrap();
}

#[test]
fn unsupported_samples_hide_controls_and_reject_stale_clicks_and_hotkeys() {
    run(|input, app| {
        supported(app);
        draw(app, 160, 48);
        let stale = app
            .controls
            .iter()
            .find(|(_, key)| *key == KeyCode::Char('b'))
            .unwrap()
            .0;
        app.snapshot.core_times.clear();
        app.snapshot.cpu_breakdown = [("user".into(), 30.), ("system".into(), 40.)].into();
        click(input, app, stale);
        assert_eq!(app.config.display.cpu_cores, "bars");
        press(input, app, KeyCode::Char('b'));
        assert_eq!(app.config.display.cpu_cores, "bars");
        let terminal = draw(app, 160, 48);
        let rendered = text(&terminal, terminal.backend().buffer().area);
        assert!(rendered.contains("Per-core history"));
        assert!(!rendered.contains('▕'));
        assert!(
            !app.controls
                .iter()
                .any(|(_, key)| *key == KeyCode::Char('b'))
        );
        press(input, app, KeyCode::F(5));
        assert!(
            !app.ui_menu
                .as_ref()
                .unwrap()
                .items
                .iter()
                .any(|(label, _)| label.starts_with("CPU cores:"))
        );
    });
}
