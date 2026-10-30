use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Color};
use rstop::{
    app::{App, View},
    config::{Config, LayoutNode},
    model::{CoreTime, Snapshot},
    ui,
};

fn sample(cores: usize) -> Snapshot {
    Snapshot {
        cores: vec![45.3; cores],
        core_times: vec![
            CoreTime {
                nice: 5.,
                user: 15.,
                system: 10.,
                iowait: 20.,
                steal: 5.
            };
            cores
        ],
        cpu_breakdown: [
            ("nice".into(), 1.2),
            ("user".into(), 21.6),
            ("system".into(), 0.8),
            ("steal".into(), 0.),
            ("iowait".into(), 0.2),
        ]
        .into(),
        ..Default::default()
    }
}
fn render(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    terminal.backend().buffer().clone()
}
fn rows(buffer: &Buffer) -> Vec<String> {
    buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect())
        .collect()
}
fn cpu_app(snapshot: Snapshot) -> App {
    let mut config = Config::default();
    config.display.overview_panel = "cpu".into();
    let mut app = App::new(config);
    app.ingest(snapshot);
    app
}

#[test]
fn bars_use_the_core_label_value_and_preserve_temperature_and_health_colors() {
    let mut snapshot = sample(4);
    snapshot.cores = vec![45.3, 70., 90., 100.];
    snapshot.core_temperatures = vec![Some(50.), None, Some(60.), Some(65.)];
    let mut app = cpu_app(snapshot);
    let buffer = render(&mut app, 200, 48);
    let text = rows(&buffer).join("\n");
    assert!(text.contains(
        "Per-core usage  ▒ low 1.2%  █ user 21.6%  ▓ sys 0.8%  ▥ steal 0.0%  ░ wait 0.2%"
    ));
    for (index, value, color) in [
        (0, "45.3%", ui::palette("midnight").mint),
        (1, "70.0%", ui::palette("midnight").amber),
        (2, "90.0%", ui::palette("midnight").red),
        (3, "100.0%", ui::palette("midnight").red),
    ] {
        let (y, row) = rows(&buffer)
            .into_iter()
            .enumerate()
            .find(|(_, row)| row.contains(&format!("C{index:02}")))
            .unwrap();
        let label_pos = row.find(&format!("C{index:02}")).unwrap();
        let (bar, rest) = row[label_pos..]
            .split_once('▕')
            .unwrap()
            .1
            .split_once('▏')
            .unwrap();
        let busy = bar
            .chars()
            .filter(|c| matches!(c, '▒' | '█' | '▓' | '▥'))
            .count();
        assert_eq!(
            busy,
            (app.snapshot.cores[index] as f64 * bar.chars().count() as f64 / 100.).round() as usize
        );
        assert!(rest.trim_start().starts_with(value));
        let label_x = row[..row.find(value).unwrap()].chars().count() as u16;
        assert_eq!(buffer[(label_x, y as u16)].fg, color);
    }
    assert!(text.contains("50°C"));
    assert!(text.contains("60°C"));
}

#[test]
fn unsupported_snapshots_always_show_history_without_a_toggle_or_legend() {
    for theme in rstop::config::THEMES {
        for times in [vec![], vec![CoreTime::default()]] {
            let mut snapshot = sample(3);
            snapshot.core_times = times;
            let mut app = cpu_app(snapshot.clone());
            app.config.theme = theme.to_string();
            for elapsed in 1..4 {
                snapshot.elapsed = elapsed as f64;
                app.ingest(snapshot.clone());
            }
            let buffer = render(&mut app, 160, 48);
            let rendered = rows(&buffer);
            let header = rendered
                .iter()
                .find_map(|row| row.find("Per-core history").map(|start| &row[start..]))
                .unwrap();
            assert_eq!(header.trim_end_matches([' ', '│']), "Per-core history");
            assert!(
                !app.controls
                    .iter()
                    .any(|(_, key)| *key == crossterm::event::KeyCode::Char('b'))
            );
            assert!(
                !app.display_menu()
                    .items
                    .iter()
                    .any(|(label, _)| label.starts_with("CPU cores:"))
            );
            assert_eq!(app.config.display.cpu_cores, "bars");
            let core_rows: Vec<_> = rendered
                .iter()
                .filter(|row| row.contains("C00") || row.contains("C01"))
                .collect();
            assert!(!core_rows.is_empty());
            assert!(core_rows.iter().all(|row| !row.contains('▕')));
            assert!(
                core_rows
                    .iter()
                    .any(|row| row.chars().any(|c| (0x2800..=0x28ff).contains(&(c as u32))))
            );
        }
    }
}

#[test]
fn supported_history_mode_retains_sparklines_and_offers_bars() {
    let mut app = cpu_app(sample(2));
    app.config.display.cpu_cores = "history".into();
    let buffer = render(&mut app, 160, 48);
    let text = rows(&buffer).join("\n");
    assert!(text.contains("Per-core history"));
    assert!(text.contains("[b] Bars"));
    assert!(!text.contains('▕'));
    assert!(
        app.controls
            .iter()
            .any(|(_, key)| *key == crossterm::event::KeyCode::Char('b'))
    );
}

#[test]
fn ascii_and_no_color_keep_five_distinct_segment_glyphs() {
    for (ascii, no_color) in [(true, false), (false, true), (true, true)] {
        let mut app = cpu_app(sample(2));
        app.config.ascii = ascii;
        app.config.no_color = no_color;
        let buffer = render(&mut app, 180, 40);
        let row = rows(&buffer)
            .into_iter()
            .find(|row| row.contains("C00"))
            .unwrap();
        for glyph in if ascii {
            [':', '#', '=', '~', '.']
        } else {
            ['▒', '█', '▓', '▥', '░']
        } {
            assert!(row.contains(glyph), "{row}");
        }
        if ascii {
            assert!(buffer.content.iter().all(|c| c.symbol().is_ascii()));
        }
        if no_color {
            assert!(
                buffer
                    .content
                    .iter()
                    .all(|c| c.fg == Color::Reset && c.bg == Color::Reset)
            );
        }
    }
}

#[test]
fn cpu_cores_config_defaults_validates_cycles_and_roundtrips() {
    let mut config: Config = toml::from_str("[display]\ncpu_graph = 'user'").unwrap();
    assert_eq!(config.display.cpu_cores, "bars");
    let index = config
        .display
        .settings()
        .iter()
        .position(|(key, _)| *key == "CPU cores")
        .unwrap();
    config.display.adjust(index, true, &[], &[]);
    assert_eq!(config.display.cpu_cores, "history");
    config.validate().unwrap();
    let saved = toml::to_string(&config).unwrap();
    assert_eq!(
        toml::from_str::<Config>(&saved).unwrap().display.cpu_cores,
        "history"
    );
    config.display.adjust(index, false, &[], &[]);
    assert_eq!(config.display.cpu_cores, "bars");
    config.display.cpu_cores = "unknown".into();
    assert!(config.validate().is_err());
}

#[test]
fn bars_and_history_render_all_layouts_sizes_and_core_counts() {
    let leaf = || LayoutNode::Panel {
        panel: "cpu".into(),
    };
    let layouts = [
        None,
        Some(leaf()),
        Some(LayoutNode::Split {
            direction: "vertical".into(),
            children: vec![leaf(), leaf()],
            weights: vec![1, 3],
        }),
        Some(LayoutNode::Split {
            direction: "horizontal".into(),
            children: vec![leaf(), leaf(), leaf()],
            weights: vec![1, 1, 20],
        }),
    ];
    for mode in ["bars", "history"] {
        for count in [0, 1, 2, 7, 64, 256] {
            for layout in ["balanced", "compute", "storage", "minimal"] {
                for custom in &layouts {
                    let mut app = App::new(Config {
                        layout: layout.into(),
                        custom_layout: custom.clone(),
                        ..Default::default()
                    });
                    app.config.display.cpu_cores = mode.into();
                    for elapsed in 0..3 {
                        let mut snapshot = sample(count);
                        snapshot.elapsed = elapsed as f64;
                        app.ingest(snapshot);
                    }
                    app.set_view(View::Overview);
                    for (width, height) in [
                        (60, 18),
                        (60, 120),
                        (80, 24),
                        (100, 30),
                        (160, 48),
                        (300, 18),
                        (300, 120),
                    ] {
                        render(&mut app, width, height);
                        let times = std::mem::take(&mut app.snapshot.core_times);
                        render(&mut app, width, height);
                        app.snapshot.core_times = times;
                    }
                }
            }
        }
    }
}
