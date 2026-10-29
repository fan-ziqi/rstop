mod commands;
mod export;
mod remote;
mod runtime;

use anyhow::{Context, Result};
use clap::Parser;
use rstop::{
    STOP,
    app::{App, View},
    collector::Collector,
    config::Config,
    discovery,
};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "One terminal. Your whole system. CPU, GPU, containers and services.",
    after_help = "Monitor another host over SSH: rstop [OPTIONS] [user@]host[:port] [gpu|network]\nAlso accepts ssh://[user@]host[:port] and ~/.ssh/config aliases."
)]
pub struct Args {
    #[command(subcommand)]
    command: Option<commands::Command>,
    /// Sampling interval in milliseconds (250..=10000)
    #[arg(global = true, short,long,value_parser=clap::value_parser!(u64).range(250..=10000))]
    interval: Option<u64>,
    /// Color theme (also adjustable in F2 settings)
    #[arg(global = true, long, value_parser = clap::builder::PossibleValuesParser::new(rstop::config::THEMES.iter().copied()))]
    theme: Option<String>,
    #[arg(global = true, long,value_parser=["overview","processes","gpu","storage","network","system","containers","services","explorer","history","events"])]
    view: Option<String>,
    /// Initial GPU display; device opens Device info (history aliases summary)
    #[arg(global = true, long, default_value="summary", value_parser=["summary","history","device"])]
    gpu_panel: String,
    /// Alternate configuration file
    #[arg(global = true, long)]
    config: Option<PathBuf>,
    /// Disable GPU driver queries
    #[arg(global = true, long)]
    no_gpu: bool,
    /// Print a JSON snapshot, then exit (includes process commands)
    #[arg(global = true, long, hide = true)]
    snapshot: bool,
    /// Number of samples; emits newline-delimited JSON, or fills SVG graph history
    #[arg(global = true, long,value_parser=clap::value_parser!(u32).range(1..=86400))]
    samples: Option<u32>,
    /// Render real collected data to an SVG, without opening a terminal
    #[arg(global = true, long, hide = true)]
    export_svg: Option<PathBuf>,
    #[arg(global = true, long,default_value_t=160,value_parser=clap::value_parser!(u16).range(60..=300))]
    width: u16,
    #[arg(global = true, long,default_value_t=48,value_parser=clap::value_parser!(u16).range(18..=120))]
    height: u16,
    /// Write preferences to the configuration file, then exit
    #[arg(global = true, long, hide = true)]
    write_config: bool,
    #[arg(global = true, long, hide = true)]
    record: Option<PathBuf>,
    #[arg(global = true, long, hide = true)]
    csv: Option<PathBuf>,
    #[arg(global = true, long, hide = true)]
    replay: Option<PathBuf>,
    #[arg(global = true, long)]
    ascii: bool,
    #[arg(global = true, long)]
    no_color: bool,
    #[arg(global = true, long)]
    threads: bool,
    #[arg(global = true, long)]
    process_network: bool,
    #[arg(global = true, long)]
    read_only: bool,
    /// Preload all pages and keep their background sampling active
    #[arg(global = true, long)]
    full_sampling: bool,
    #[arg(global = true, long,value_parser=["balanced","compute","storage","minimal"])]
    layout: Option<String>,
    /// Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit
    #[arg(global = true, long,value_parser=clap::value_parser!(u32).range(1..=128), hide = true)]
    select_gpu: Option<u32>,
    #[arg(global = true, long, default_value_t = 0)]
    min_free_mib: u64,
    /// Minimum total VRAM for GPU selection
    #[arg(global = true, long, default_value_t = 0)]
    min_total_mib: u64,
    /// GPU selection ceiling; devices with unknown utilization are excluded
    #[arg(global = true, long, value_parser=clap::value_parser!(u32).range(0..=100))]
    max_gpu_utilization: Option<u32>,
    /// GPU selection memory-controller utilization ceiling
    #[arg(global = true, long, value_parser=clap::value_parser!(u32).range(0..=100))]
    max_memory_utilization: Option<u32>,
    /// Disable configured probes and alert commands
    #[arg(global = true, long)]
    no_config_exec: bool,
}

fn main() -> Result<()> {
    let mut args = Args::parse();
    ctrlc::set_handler(|| STOP.store(true, Ordering::Relaxed))
        .context("Cannot install shutdown handler")?;
    if let Some(commands::Command::Remote(external)) = &args.command {
        let (destination, args) = remote::split(external);
        return remote::run(&destination, args);
    }
    if commands::dispatch(&mut args)? {
        return Ok(());
    }
    let path = args.config.clone().unwrap_or_else(Config::path);
    let mut config = Config::load(&path)?;
    if let Some(interval) = args.interval {
        config.interval_ms = interval;
    }
    if let Some(theme) = &args.theme {
        config.theme = theme.clone();
    }
    config.collect_gpu &= !args.no_gpu;
    config.ascii |= args.ascii;
    config.no_color |= args.no_color;
    config.show_threads |= args.threads;
    config.process_network |= args.process_network;
    if args.read_only || args.no_config_exec {
        config.config_exec = false;
    }
    if let Some(layout) = &args.layout {
        config.layout = layout.clone();
        config.custom_layout = None;
    }
    config.validate()?;
    if let Some(count) = args.select_gpu {
        let (devices, _) = rstop::gpu::GpuCollector::new(true).collect();
        let gpus = rstop::gpu::select(
            &devices,
            &rstop::gpu::Selection {
                min_free_bytes: args.min_free_mib.saturating_mul(1024 * 1024),
                min_total_bytes: args.min_total_mib.saturating_mul(1024 * 1024),
                max_gpu_utilization: args.max_gpu_utilization,
                max_memory_utilization: args.max_memory_utilization,
            },
        );
        anyhow::ensure!(
            gpus.len() >= count as usize,
            "Not enough readable CUDA devices with requested free memory"
        );
        println!(
            "CUDA_VISIBLE_DEVICES={}",
            gpus.iter()
                .take(count as usize)
                .map(|g| g.uuid.clone())
                .collect::<Vec<_>>()
                .join(",")
        );
        return Ok(());
    }
    if let Some(replay) = &args.replay {
        for output in [
            args.record.as_ref(),
            args.csv.as_ref(),
            args.export_svg.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            anyhow::ensure!(
                replay.canonicalize().ok() != output.canonicalize().ok() || !output.exists(),
                "Output would overwrite replay input"
            );
        }
    }
    if args.write_config {
        config.save(&path)?;
        println!("Saved {}", path.display());
        return Ok(());
    }
    if args.snapshot || args.samples.is_some() || args.export_svg.is_some() || args.csv.is_some() {
        return headless(&args, config);
    }
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "Interactive mode needs a terminal. Use 'rstop snapshot' or 'rstop sample --samples N' for JSON."
    );
    runtime::run(&args, config, path)
}

pub fn gpu_section(args: &Args) -> usize {
    match args.gpu_panel.as_str() {
        "history" => 0,
        "device" => 2,
        _ => 0,
    }
}

pub fn wanted_view(args: &Args, config: &Config) -> View {
    match args.view.as_deref() {
        Some("processes") => View::Processes,
        Some("gpu") => View::Gpu,
        Some("storage") => View::Storage,
        Some("network") => View::Network,
        Some("system") => View::System,
        Some("containers") => View::Containers,
        Some("services") => View::Services,
        Some("explorer") => View::System,
        Some("history") => View::Overview,
        Some("events") => View::Overview,
        _ => [
            View::Overview,
            View::Processes,
            View::Gpu,
            View::Storage,
            View::Network,
            View::System,
            View::Containers,
            View::Services,
            View::System,
            View::Overview,
            View::Overview,
            View::Processes, // Previous saved Applications preference
        ][config.view.min(11)],
    }
}

fn headless(args: &Args, config: Config) -> Result<()> {
    if let Some(path) = &args.replay {
        let snapshots = rstop::recording::replay(path)?;
        let mut app = App::new(config.clone());
        app.gpu_section = gpu_section(args);
        app.replay = true;
        let mut csv = args.csv.as_ref().map(std::fs::File::create).transpose()?;
        let mut recorder = args
            .record
            .as_ref()
            .map(|p| rstop::recording::Recorder::new(p, config.record_max_mib))
            .transpose()?;
        for (index, snapshot) in snapshots
            .into_iter()
            .take(
                args.samples
                    .unwrap_or(if args.snapshot { 1 } else { u32::MAX }) as usize,
            )
            .enumerate()
        {
            if let Some(recorder) = &mut recorder {
                recorder.write(&snapshot)?;
            }
            if let Some(writer) = &mut csv {
                rstop::recording::csv(&snapshot, writer, index == 0)?;
            }
            if args.export_svg.is_none() && args.csv.is_none() {
                println!("{}", serde_json::to_string(&snapshot)?);
            }
            app.ingest(snapshot);
        }
        if let Some(path) = &args.export_svg {
            app.status = "Local replay · recorded host telemetry".into();
            app.view = wanted_view(args, &config);
            export::svg(path, args.width, args.height, &mut app)?;
        }
        return Ok(());
    }
    let mut recorder = args
        .record
        .as_ref()
        .map(|path| rstop::recording::Recorder::new(path, config.record_max_mib))
        .transpose()?;
    let mut csv = args.csv.as_ref().map(std::fs::File::create).transpose()?;
    let mut collector = Collector::new(config.collect_gpu);
    let (connection_cache, connection_worker) = rstop::connections::worker();
    let mut rates = discovery::InventoryRates::default();
    let mut first_inventory = discovery::discover_with_config(&config);
    rates.apply(&mut first_inventory);
    let inventory = std::sync::Arc::new(std::sync::RwLock::new(first_inventory));
    let worker = if args.samples.unwrap_or(1) > 1 {
        let inventory = inventory.clone();
        let discovery_config = config.clone();
        Some(std::thread::spawn(move || {
            while !STOP.load(Ordering::Relaxed) {
                wait_until(Instant::now() + Duration::from_secs(5));
                if STOP.load(Ordering::Relaxed) {
                    break;
                }
                let mut data = discovery::discover_with_config(&discovery_config);
                rates.apply(&mut data);
                if let Ok(mut current) = inventory.write() {
                    *current = data;
                }
            }
        }))
    } else {
        None
    };
    let mut app = App::new(config.clone());
    app.gpu_section = gpu_section(args);
    let mut stdout = io::BufWriter::new(io::stdout());
    let mut deadline = Instant::now() + Duration::from_millis(config.interval_ms);
    for index in 0..args.samples.unwrap_or(1) {
        wait_until(deadline);
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        let mut snapshot = collector.collect(&config);
        if let Ok(sample) = connection_cache.read() {
            snapshot.connections = sample.clone();
        }
        for row in &mut snapshot.connections.connections {
            if row.process_names.is_empty() {
                row.process_names = row
                    .pids
                    .iter()
                    .filter_map(|pid| {
                        snapshot
                            .processes
                            .iter()
                            .find(|p| p.pid == *pid)
                            .map(|p| p.name.clone())
                    })
                    .collect();
            }
        }
        if let Ok(current) = inventory.read() {
            snapshot.apply_inventory(current.clone());
        }
        rstop::tags::annotate(&mut snapshot);
        if let Some(recorder) = &mut recorder {
            recorder.write(&snapshot)?;
        }
        if let Some(writer) = &mut csv {
            rstop::recording::csv(&snapshot, writer, index == 0)?;
        }
        if args.export_svg.is_none() && args.csv.is_none() {
            if args.samples.is_some() {
                serde_json::to_writer(&mut stdout, &snapshot)?;
            } else {
                serde_json::to_writer_pretty(&mut stdout, &snapshot)?;
            }
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
        app.ingest(snapshot);
        deadline = Instant::now() + Duration::from_millis(config.interval_ms);
    }
    STOP.store(true, Ordering::Relaxed);
    let _ = connection_worker.join();
    if let Some(worker) = worker {
        let _ = worker.join();
    }
    if let Some(path) = &args.export_svg {
        app.view = wanted_view(args, &config);
        if !app.views().contains(&app.view) {
            app.view = View::Overview;
        }
        app.status = "Real host telemetry · rstop".into();
        export::svg(path, args.width, args.height, &mut app)?;
        eprintln!("Saved {}", path.display());
    }
    Ok(())
}

pub fn wait_until(deadline: Instant) {
    while Instant::now() < deadline && !STOP.load(Ordering::Relaxed) {
        std::thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
    }
}
