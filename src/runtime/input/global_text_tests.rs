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

fn copy_visible(input: &mut super::Input<'_>, app: &mut App, needle: &str) {
    draw(app);
    let region = app
        .text_regions
        .iter()
        .find(|region| region.is_screen())
        .unwrap();
    let area = region.buffer.area;
    let (column, row) = (area.y..area.bottom())
        .find_map(|row| {
            let line: String = (area.x..area.right())
                .map(|x| region.buffer[(x, row)].symbol())
                .collect();
            line.find(needle).map(|index| {
                (
                    area.x + ratatui::text::Line::raw(&line[..index]).width() as u16,
                    row,
                )
            })
        })
        .unwrap_or_else(|| panic!("Missing visible text: {needle}"));
    let mut mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    input.mouse(app, mouse);
    mouse.kind = MouseEventKind::Drag(MouseButton::Left);
    mouse.column += ratatui::text::Line::raw(needle).width().saturating_sub(1) as u16;
    input.mouse(app, mouse);
    draw(app);
    mouse.kind = MouseEventKind::Up(MouseButton::Left);
    input.mouse(app, mouse);
    assert!(input.key(
        app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    ));
    assert_eq!(app.clipboard_request.take().as_deref(), Some(needle));
}

#[test]
fn copy_works_across_resource_pages_tables_and_every_popup_surface() {
    use rstop::{
        app::{SignalTarget, UiMenu, View},
        model::Gpu,
    };
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
        run(|input, app| {
            app.snapshot.hostname = "copy-host".into();
            app.set_view(view);
            copy_visible(input, app, "copy-host");
        });
    }
    run(|input, app| {
        copy_visible(input, app, "worker");
        assert!(app.selected_process.is_some());
        assert!(!app.selection_open);
    });
    for popup in [
        "gpu", "settings", "menu", "prompt", "signal", "update", "main",
    ] {
        run(|input, app| {
            let needle = match popup {
                "gpu" => {
                    app.snapshot.gpus.push(Gpu {
                        uuid: "copy-gpu".into(),
                        extra: [("pci_bus_id".into(), serde_json::json!("0000:01:00.0"))].into(),
                        ..Default::default()
                    });
                    app.set_view(View::Gpu);
                    app.focus_panel(2);
                    app.filter = "pci".into();
                    "0000:01:00.0"
                }
                "settings" => {
                    app.settings = true;
                    "Settings"
                }
                "menu" => {
                    app.ui_menu = Some(UiMenu::new(
                        "copy-menu",
                        vec![("Action".into(), KeyCode::Null)],
                        0,
                    ));
                    "copy-menu"
                }
                "prompt" => {
                    app.prompt = Some(("copy prompt".into(), "copy-input".into(), None));
                    "copy-input"
                }
                "signal" => {
                    app.signal = Some(SignalTarget {
                        pid: 42,
                        start_time: 0,
                        name: "copy-signal-target".into(),
                        signal: sysinfo::Signal::Term,
                    });
                    "copy-signal-target"
                }
                "update" => {
                    app.update_notice = Some(rstop::updates::Notice::checking());
                    "rstop update"
                }
                _ => {
                    app.main_menu = Some(Default::default());
                    "rstop"
                }
            };
            copy_visible(input, app, needle);
            assert_eq!(
                app.text_regions
                    .iter()
                    .find(|region| region.is_screen())
                    .unwrap()
                    .buffer
                    .area,
                app.popup_rect
            );
            assert!(app.pending_plan.is_none());
            assert!(input.queued_key.is_none());
        });
    }
}

#[test]
fn global_selection_keeps_real_mouse_release_double_clicks_working() {
    use rstop::{app::View, model::Gpu};
    run(|input, app| {
        app.snapshot.gpus = (0..2)
            .map(|id| Gpu {
                id,
                uuid: format!("gpu-{id}"),
                ..Default::default()
            })
            .collect();
        app.set_view(View::Gpu);
        draw(app);
        let card = app.gpu_ranges[1].0;
        let mut mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: card.x + 4,
            row: card.y + 1,
            modifiers: KeyModifiers::NONE,
        };
        for click in 0..2 {
            mouse.kind = MouseEventKind::Down(MouseButton::Left);
            input.mouse(app, mouse);
            mouse.kind = MouseEventKind::Up(MouseButton::Left);
            input.mouse(app, mouse);
            draw(app);
            assert_eq!(app.gpu_info_open(), click == 1);
        }
        assert_eq!(app.selected_gpu, 1);
        assert!(app.text_selection.selected_text().is_none());
    });
}
