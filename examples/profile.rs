//! Stage timings for the live pipeline, without a terminal emulator.
//! cargo run --release --example profile -- 10
use ratatui::{Terminal, backend::TestBackend};
use rstop::{app::App, collector::Collector, config::Config, connections, discovery, tags, ui};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[cfg(unix)]
fn cpu_ms() -> f64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time);
    }
    time.tv_sec as f64 * 1000. + time.tv_nsec as f64 / 1e6
}
#[cfg(not(unix))]
fn cpu_ms() -> f64 {
    0.
}

fn measure<T>(
    name: &'static str,
    stats: &mut BTreeMap<&'static str, Vec<(f64, f64)>>,
    f: impl FnOnce() -> T,
) -> T {
    let wall = Instant::now();
    let cpu = cpu_ms();
    let value = f();
    stats
        .entry(name)
        .or_default()
        .push((cpu_ms() - cpu, wall.elapsed().as_secs_f64() * 1000.));
    value
}

fn main() -> anyhow::Result<()> {
    let rounds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    let config = Config {
        collect_gpu: !std::env::args().any(|s| s == "--no-gpu"),
        ..Config::default()
    };
    let mut stats = BTreeMap::new();
    let inventory = measure("discovery", &mut stats, || {
        discovery::discover_with_config(&config)
    });
    let mut sockets = connections::Collector::default();
    let mut collector = Collector::new(config.collect_gpu);
    let mut app = App::new(config.clone());
    let mut terminal = Terminal::new(TestBackend::new(160, 48))?;
    std::thread::sleep(Duration::from_secs(1));
    for _ in 0..rounds {
        let began = Instant::now();
        let connections = measure("connections", &mut stats, || sockets.collect());
        let mut sample = measure("collector", &mut stats, || collector.collect(&config));
        measure("merge", &mut stats, || {
            sample.apply_inventory(inventory.clone());
            sample.connections = connections;
        });
        let applications = measure("sampler tags", &mut stats, || tags::annotate(&mut sample));
        measure("app ingest", &mut stats, || {
            app.ingest_annotated(sample, applications)
        });
        measure("draw", &mut stats, || {
            terminal.draw(|f| ui::draw(f, &mut app))
        })?;
        if let Some(wait) = Duration::from_secs(1).checked_sub(began.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    let summary: BTreeMap<_,_> = stats.into_iter().map(|(name, rows)| {
        let n = rows.len() as f64;
        (name,serde_json::json!({"calls":rows.len(),"cpu_ms_mean": rows.iter().map(|r|r.0).sum::<f64>()/n,"wall_ms_mean":rows.iter().map(|r|r.1).sum::<f64>()/n}))
    }).collect();
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
