//! Export rstop's actual terminal buffer for the website. No alternate UI.
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use rstop::{
    app::{App, Sort, View},
    config::Config,
    ui,
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::Path};

fn colors(name: &str) -> Vec<Color> {
    let p = ui::palette(name);
    vec![
        p.bg,
        p.panel,
        p.border,
        p.muted,
        p.text,
        p.mint,
        p.blue,
        p.purple,
        p.amber,
        p.red,
        p.selection,
    ]
}
fn rgb(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".into(),
        Color::White => "#ffffff".into(),
        Color::Gray => "#c0c0c0".into(),
        Color::DarkGray => "#808080".into(),
        Color::Red => "#800000".into(),
        Color::Green => "#008000".into(),
        Color::Blue => "#000080".into(),
        Color::Yellow => "#808000".into(),
        Color::Cyan => "#008080".into(),
        Color::Magenta => "#800080".into(),
        Color::LightRed => "#ff0000".into(),
        Color::LightGreen => "#00ff00".into(),
        Color::LightBlue => "#0000ff".into(),
        Color::LightYellow => "#ffff00".into(),
        Color::LightCyan => "#00ffff".into(),
        Color::LightMagenta => "#ff00ff".into(),
        other => panic!("Unsupported native color: {other:?}"),
    }
}
fn token(color: Color, palette: &[Color], background: bool) -> Value {
    if color == Color::Reset {
        return json!(if background { 0 } else { 4 });
    }
    palette
        .iter()
        .position(|c| *c == color)
        .map(|i| json!(i))
        .unwrap_or_else(|| json!(rgb(color)))
}
fn style(cell: &ratatui::buffer::Cell, palette: &[Color]) -> Value {
    json!([
        token(cell.fg, palette, false),
        token(cell.bg, palette, true),
        cell.modifier.bits()
    ])
}
fn make_app(snapshots: &[rstop::model::Snapshot], view: View, theme: &str, window: u64) -> App {
    let mut app = App::new(Config {
        theme: theme.into(),
        time_window_seconds: window,
        check_updates: false,
        config_exec: false,
        ..Config::default()
    });
    // Ingest only the fictional fixture. Never run a collector, probe or command.
    for sample in snapshots.iter().take(96) {
        app.ingest(sample.clone());
    }
    app.set_view(view);
    app.status = "Demo session".into();
    app
}
fn walkthrough(app: &mut App, step: usize) -> &'static str {
    match step {
        0..=2 => {
            app.begin_filter();
            app.filter = ["c", "cpu:", "cpu:>10"][step].into();
            "Filter processes"
        }
        3..=5 => {
            app.filter = "cpu:>10".into();
            app.editing_filter = false;
            "Filter processes"
        }
        6..=8 => {
            app.filter.clear();
            app.editing_filter = false;
            app.sort = Sort::Memory;
            app.reverse = false;
            "Sort by memory"
        }
        9..=11 => {
            app.tree = true;
            "Show process tree"
        }
        12..=14 => {
            app.tree = false;
            app.selected = 0;
            app.remember_selection();
            "Select a process"
        }
        _ => {
            app.open_selection();
            "Inspect process details"
        }
    }
}
fn bindings(app: &App, include_models: bool) -> Vec<Value> {
    let models: Vec<(String, Value)> = match app.view {
        View::Processes => app
            .process_rows()
            .into_iter()
            .map(|(i, _)| {
                let process = &app.snapshot.processes[i];
                (process.pid.to_string(), json!(process))
            })
            .collect(),
        View::Network => app
            .filtered_connections()
            .into_iter()
            .map(|row| (row.id.clone(), json!(row)))
            .collect(),
        View::Containers => app
            .filtered_containers()
            .into_iter()
            .map(|row| (row.id.clone(), json!(row)))
            .collect(),
        View::Services => app
            .filtered_services()
            .into_iter()
            .map(|row| (row.name.clone(), json!(row)))
            .collect(),
        _ => vec![],
    };
    models
        .into_iter()
        .skip(app.table_offset)
        .take(app.table_rect.height as usize)
        .enumerate()
        .map(|(i, (id, model))| {
            if include_models {
                json!({"id":id,"y":app.table_rect.y+i as u16,"model":model})
            } else {
                json!({"id":id,"y":app.table_rect.y+i as u16})
            }
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let samples = rstop::recording::replay(Path::new(&args[1]))?;
    let output = Path::new(&args[2]);
    std::fs::create_dir_all(output)?;
    let mut query_app = make_app(&samples, View::Processes, "midnight", 120);
    query_app.ingest(samples[96].clone());
    let query_cases:Vec<_>=["","python","name:node","cpu:>10","mem:>1000","!name:rust","pid:2400 | pid:2402","/python|rust/","name:\"redis-server\"","badfield:value","\"unclosed"].into_iter().map(|query|{
        query_app.filter=query.into();
        json!({"query":query,"ids":query_app.process_rows().into_iter().map(|(i,_)|query_app.snapshot.processes[i].pid.to_string()).collect::<Vec<_>>()})
    }).collect();
    std::fs::write(
        output.join("query-cases.json"),
        serde_json::to_vec(&query_cases)?,
    )?;
    if args.get(3).is_some_and(|arg| arg == "--queries-only") {
        return Ok(());
    }
    let themes = rstop::config::THEMES;
    let named_palettes: serde_json::Map<String, Value> = themes
        .iter()
        .map(|theme| {
            (
                theme.to_string(),
                json!(colors(theme).into_iter().map(rgb).collect::<Vec<_>>()),
            )
        })
        .collect();
    let views = [
        View::Overview,
        View::Processes,
        View::Gpu,
        View::Storage,
        View::Network,
        View::System,
        View::Containers,
        View::Services,
    ];
    let mut checked_cells = 0usize;
    for (size, width, height, window) in [
        ("wide", 160u16, 44u16, 120u64),
        ("medium", 100, 40, 120),
        ("compact", 64, 36, 60),
    ] {
        for view in views {
            let mut rows: Vec<Value> = vec![];
            let mut row_ids = HashMap::new();
            let mut styles: Vec<Value> = vec![];
            let mut style_ids = HashMap::new();
            let mut raw_styles: Vec<[(Color, Color, u16); 9]> = vec![];
            let theme_colors: Vec<_> = themes.iter().map(|theme| colors(theme)).collect();
            let mut sequences = serde_json::Map::new();
            let mut tabs = vec![];
            let mut scenarios = vec!["live".to_string()];
            let selectable = make_app(&samples, view, "midnight", window);
            if matches!(
                view,
                View::Processes | View::Network | View::Containers | View::Services
            ) {
                scenarios.push("empty".into());
                for index in 0..selectable.row_count() {
                    scenarios.push(format!("inspect-{index}"));
                }
            }
            if view == View::Processes {
                scenarios.extend(["walkthrough".into(), "tree".into()]);
                for index in 0..selectable.row_count() {
                    scenarios.push(format!("tree-inspect-{index}"));
                }
            }
            for scenario in scenarios {
                let mut apps: Vec<_> = themes
                    .iter()
                    .map(|theme| make_app(&samples, view, theme, window))
                    .collect();
                let mut terminals: Vec<_> = themes
                    .iter()
                    .map(|_| Terminal::new(TestBackend::new(width, height)).unwrap())
                    .collect();
                let mut frames = vec![];
                let count = if scenario == "walkthrough" { 18 } else { 24 };
                for index in 0..count {
                    let mut phase = "";
                    for (app, terminal) in apps.iter_mut().zip(terminals.iter_mut()) {
                        app.ingest(samples[96 + index].clone());
                        if scenario == "walkthrough" {
                            phase = walkthrough(app, index);
                        }
                        if scenario == "tree" {
                            app.tree = true;
                        }
                        if scenario.starts_with("tree-") {
                            app.tree = true;
                        }
                        if scenario == "empty" {
                            app.filter = "__no_match__".into();
                        }
                        if let Some(selected) = scenario
                            .strip_prefix("inspect-")
                            .or_else(|| scenario.strip_prefix("tree-inspect-"))
                        {
                            app.selected = selected.parse().unwrap();
                            app.remember_selection();
                            app.open_selection();
                        }
                        terminal.draw(|frame| ui::draw(frame, app))?;
                    }
                    if tabs.is_empty() {
                        tabs = apps[0]
                            .tab_ranges
                            .iter()
                            .map(|(r, v)| {
                                json!([r.x, r.y, r.width, r.height, v.name().to_lowercase()])
                            })
                            .collect();
                    }
                    let buffer = terminals[0].backend().buffer();
                    let mut frame_rows = vec![];
                    for y in 0..height {
                        let mut runs: Vec<Value> = vec![];
                        let mut current: Option<(usize, String)> = None;
                        for x in 0..width {
                            let cell = &buffer[(x, y)];
                            let key: [(Color, Color, u16); 9] = std::array::from_fn(|i| {
                                let themed_cell = &terminals[i].backend().buffer()[(x, y)];
                                assert_eq!(
                                    themed_cell.symbol(),
                                    cell.symbol(),
                                    "Theme geometry differs: {}",
                                    themes[i]
                                );
                                checked_cells += 1;
                                (themed_cell.fg, themed_cell.bg, themed_cell.modifier.bits())
                            });
                            let id = *style_ids.entry(key).or_insert_with(|| {
                                let id = styles.len();
                                styles.push(json!(
                                    terminals
                                        .iter()
                                        .enumerate()
                                        .map(|(i, terminal)| style(
                                            &terminal.backend().buffer()[(x, y)],
                                            &theme_colors[i]
                                        ))
                                        .collect::<Vec<_>>()
                                ));
                                raw_styles.push(key);
                                id
                            });
                            let symbol = cell.symbol();
                            // Fixtures contain single-cell terminal symbols, including braille.
                            assert_eq!(
                                symbol.chars().count(),
                                1,
                                "Non-cell symbol at {x},{y}: {symbol:?}"
                            );
                            match current.as_mut() {
                                Some((last, text)) if *last == id => text.push_str(symbol),
                                _ => {
                                    if let Some((last, text)) = current.take() {
                                        runs.push(json!([last, text]));
                                    }
                                    current = Some((id, symbol.into()));
                                }
                            }
                        }
                        if let Some((last, text)) = current.take() {
                            runs.push(json!([last, text]));
                        }
                        // Round-trip every character and style against the actual ratatui buffer.
                        let mut x = 0;
                        for run in &runs {
                            let id = run[0].as_u64().unwrap() as usize;
                            for symbol in run[1].as_str().unwrap().chars() {
                                assert_eq!(buffer[(x, y)].symbol(), symbol.to_string());
                                let cell = &buffer[(x, y)];
                                assert_eq!(
                                    (cell.fg, cell.bg, cell.modifier.bits()),
                                    raw_styles[id][0]
                                );
                                checked_cells += 1;
                                x += 1;
                            }
                        }
                        assert_eq!(x, width);
                        let row = json!(runs);
                        let key = row.to_string();
                        let id = *row_ids.entry(key).or_insert_with(|| {
                            let id = rows.len();
                            rows.push(row);
                            id
                        });
                        frame_rows.push(id);
                    }
                    let app = &apps[0];
                    let rect = |r: ratatui::layout::Rect| json!([r.x, r.y, r.width, r.height]);
                    frames.push(json!({"rows":frame_rows,"phase":phase,"bindings":bindings(app,scenario=="live"),"table":rect(app.table_rect),"filter":rect(app.filter_rect),
                        "sort_headers":app.sort_headers.iter().map(|(r,s)|json!([rect(*r),s.name()])).collect::<Vec<_>>() }));
                }
                sequences.insert(scenario.into(), json!(frames));
            }
            let file = output.join(format!("{}-{size}.json", view.name().to_lowercase()));
            std::fs::write(
                &file,
                serde_json::to_vec(
                    &json!({"width":width,"height":height,"palettes":named_palettes,"styles":styles,"rows":rows,"sequences":sequences,"tabs":tabs}),
                )?,
            )?;
            println!("{}: {} bytes", file.display(), file.metadata()?.len());
        }
    }
    println!(
        "Verified {checked_cells} native cells, including all nine themes at all three sizes."
    );
    // Bit assignments are exported rather than duplicated in JavaScript.
    std::fs::write(
        output.join("renderer.json"),
        serde_json::to_vec(&json!({
            "version":env!("CARGO_PKG_VERSION"),"renderer":"rstop::ui::draw","fictional_data":true,"verified_cells":checked_cells,
            "flags":{"bold":Modifier::BOLD.bits(),"dim":Modifier::DIM.bits(),"italic":Modifier::ITALIC.bits(),"underline":Modifier::UNDERLINED.bits(),"reverse":Modifier::REVERSED.bits()}
        }))?,
    )?;
    Ok(())
}
