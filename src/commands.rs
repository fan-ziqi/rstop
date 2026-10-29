use crate::Args;
use anyhow::Result;
use clap::{CommandFactory, Subcommand, ValueEnum};
use std::{io, path::PathBuf};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open the interactive system monitor (also the default command)
    Monitor,
    /// Print one JSON snapshot, including process command lines
    Snapshot,
    /// Print newline-delimited JSON samples (ten by default)
    Sample,
    /// Record telemetry while monitoring; --samples selects headless recording
    Record { file: PathBuf },
    /// Replay a local recording; --samples selects JSON output
    Replay { file: PathBuf },
    /// Export live telemetry or a recording to CSV or SVG
    Export {
        #[arg(value_enum)]
        format: ExportFormat,
        output: PathBuf,
        #[arg(long)]
        input: Option<PathBuf>,
    },
    /// Open the GPU view or select CUDA devices
    Gpu {
        #[command(subcommand)]
        command: Option<GpuCommand>,
    },
    /// Open connection monitoring, test a target, or capture packets
    Network {
        #[command(subcommand)]
        command: Option<NetworkCommand>,
    },
    /// Explain resource problems, recent trends and useful next steps
    Doctor {
        /// Diagnose an existing recording without inspecting this machine
        #[arg(long)]
        input: Option<PathBuf>,
        /// Print structured findings and the explanation as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print the configuration path or write current preferences
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    /// Generate shell completions without reading configuration
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Print the manual page without reading configuration
    Man,
    /// Update this installer-managed copy of rstop
    Update {
        /// Check availability without installing
        #[arg(long)]
        check: bool,
    },
    /// Any other first word is an SSH destination for remote monitoring
    #[command(external_subcommand)]
    Remote(Vec<String>),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ExportFormat {
    Csv,
    Svg,
}

#[derive(Debug, Subcommand)]
pub enum GpuCommand {
    /// Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs
    Select {
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=128))]
        count: u32,
    },
}

#[derive(Debug, Subcommand)]
pub enum NetworkCommand {
    /// List sockets and their owners; samples TCP counters twice for rates
    Connections {
        /// AND terms: port:443 process:python state:LISTEN pid:42 !proto:udp
        #[arg(long, default_value = "")]
        filter: String,
        #[arg(long)]
        json: bool,
        /// Choose a network field; missing values always sort last
        #[arg(long, value_enum, default_value = "traffic")]
        sort: rstop::network_sort::ConnectionSort,
        /// Reverse the chosen field's default direction
        #[arg(long)]
        reverse: bool,
    },
    /// Check an HTTP(S) URL or TCP host:port, with bounded timeouts
    Check { target: String },
    /// Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)
    Capture {
        output: PathBuf,
        #[arg(long, default_value = if cfg!(target_os="linux") {"any"} else {"en0"})]
        interface: String,
        #[arg(long, default_value = "tcp or udp or icmp or icmp6")]
        filter: String,
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=60))]
        seconds: u64,
        #[arg(long, default_value_t = 2000, value_parser = clap::value_parser!(u32).range(1..=10000))]
        packets: u32,
    },
    /// Explain and preview the first 60 headers in a local PCAP
    Read { file: PathBuf },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the configuration path
    Path,
    /// Save preferences to the configuration path
    Write,
}

/// Handle configuration-independent commands or map commands to the runtime.
pub fn dispatch(args: &mut Args) -> Result<bool> {
    let Some(command) = args.command.take() else {
        return Ok(false);
    };
    match command {
        Command::Monitor => {}
        Command::Snapshot => {
            args.snapshot = true;
            args.samples = None;
        }
        Command::Sample => args.samples = Some(args.samples.unwrap_or(10)),
        Command::Record { file } => args.record = Some(file),
        Command::Replay { file } => args.replay = Some(file),
        Command::Export {
            format,
            output,
            input,
        } => {
            args.replay = input;
            match format {
                ExportFormat::Csv => args.csv = Some(output),
                ExportFormat::Svg => args.export_svg = Some(output),
            }
        }
        Command::Gpu { command: None } => args.view = Some("gpu".into()),
        Command::Network { command: None } => args.view = Some("network".into()),
        Command::Network {
            command: Some(command),
        } => {
            network(command, args.read_only)?;
            return Ok(true);
        }
        Command::Doctor { input, json } => {
            doctor(args, input, json)?;
            return Ok(true);
        }
        Command::Gpu {
            command: Some(GpuCommand::Select { count }),
        } => {
            args.select_gpu = Some(count);
        }
        Command::Config {
            command: Some(ConfigCommand::Write),
        } => args.write_config = true,
        Command::Config {
            command: None | Some(ConfigCommand::Path),
        } => {
            println!(
                "{}",
                args.config
                    .clone()
                    .unwrap_or_else(rstop::config::Config::path)
                    .display()
            );
            return Ok(true);
        }
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Args::command(), "rstop", &mut io::stdout());
            return Ok(true);
        }
        Command::Man => {
            clap_mangen::Man::new(Args::command()).render(&mut io::stdout())?;
            return Ok(true);
        }
        Command::Update { check } => {
            update(check)?;
            return Ok(true);
        }
        Command::Remote(_) => unreachable!("main handles SSH destinations"),
    }
    Ok(false)
}

fn update(check: bool) -> Result<()> {
    let mut updater = rstop::updates::updater()?;
    let release = rstop::updates::latest()?;
    if !release.newer_than(env!("CARGO_PKG_VERSION")) {
        println!("rstop is up to date.");
    } else if check {
        println!("An update is available. Run 'rstop update' to install it.");
    } else {
        let version = rstop::updates::install_release(&mut updater, &release)?;
        println!("Updated rstop to {version}.");
    }
    Ok(())
}

fn network(command: NetworkCommand, read_only: bool) -> Result<()> {
    use rstop::{connections, network_tools};
    match command {
        NetworkCommand::Check { target } => println!("{}", network_tools::check_target(&target)?),
        NetworkCommand::Read { file } => println!("{}", network_tools::read_capture(&file)?),
        NetworkCommand::Capture {
            output,
            interface,
            filter,
            seconds,
            packets,
        } => {
            anyhow::ensure!(!read_only, "Packet capture disabled in read-only mode");
            let capture = network_tools::Capture {
                output,
                interface,
                filter,
                seconds,
                packets,
            };
            eprintln!(
                "{}",
                capture.preview().replace(
                    "y confirm · Esc cancel",
                    "Capturing as explicitly requested; Ctrl+C stops and saves."
                )
            );
            println!("{}", capture.run()?);
        }
        NetworkCommand::Connections {
            filter,
            json,
            sort,
            reverse,
        } => {
            let mut collector = connections::Collector::default();
            collector.collect();
            crate::wait_until(std::time::Instant::now() + std::time::Duration::from_secs(1));
            let mut sample = collector.collect();
            sample
                .connections
                .retain(|c| network_tools::matches(c, &filter));
            sample
                .connections
                .sort_by(|a, b| sort.compare(a, b, reverse));
            if json {
                println!("{}", serde_json::to_string_pretty(&sample)?);
            } else {
                println!(
                    "{} · sampled at {} · owners at {}\nPROCESS                   PID           PROTO  LOCAL -> DESTINATION  STATE  DOWN / UP",
                    sample.status, sample.sampled_at, sample.owners_sampled_at
                );
                for c in sample.connections {
                    println!(
                        "{:<24} {:<12} {}  {} -> {}  {}  {} / {}",
                        rstop::network_sort::ConnectionSort::Process.cell(&c),
                        rstop::network_sort::ConnectionSort::Pid.cell(&c),
                        c.protocol,
                        c.local,
                        c.remote,
                        c.state,
                        c.rx_per_sec
                            .map(|v| format!("{}/s", rstop::model::bytes(v)))
                            .unwrap_or_else(|| "—".into()),
                        c.tx_per_sec
                            .map(|v| format!("{}/s", rstop::model::bytes(v)))
                            .unwrap_or_else(|| "—".into())
                    );
                }
                for note in sample.notes {
                    eprintln!("{note}");
                }
            }
        }
    }
    Ok(())
}
fn doctor(args: &Args, input: Option<PathBuf>, json: bool) -> Result<()> {
    use rstop::{app::App, collector::Collector, config::Config};
    let mut config = Config::load(&args.config.clone().unwrap_or_else(Config::path))?;
    config.alert_commands.clear();
    config.collect_gpu &= !args.no_gpu;
    let mut app = App::new(config.clone());
    if let Some(input) = input {
        app.replay = true;
        for snapshot in rstop::recording::replay(&input)? {
            app.ingest(snapshot);
        }
    } else {
        let mut collector = Collector::new(config.collect_gpu);
        let mut connections = rstop::connections::Collector::default();
        let inventory = rstop::discovery::discover_with_config(&config);
        connections.collect();
        app.ingest(collector.collect(&config));
        crate::wait_until(
            std::time::Instant::now() + std::time::Duration::from_millis(config.interval_ms),
        );
        let mut snapshot = collector.collect(&config);
        snapshot.connections = connections.collect();
        snapshot.apply_inventory(inventory);
        app.ingest(snapshot);
    }
    app.view = crate::wanted_view(args, &config);
    let report = rstop::diagnostics::report(&app);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"sampled_at":app.snapshot.timestamp,"recorded":app.replay,"findings":rstop::diagnostics::findings(&app).into_iter().filter(|f|rstop::diagnostics::relevant(app.view,f.domain)).collect::<Vec<_>>(),"report":report})
            )?
        );
    } else {
        println!("{report}");
    }
    Ok(())
}
