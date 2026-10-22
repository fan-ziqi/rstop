use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use rstop::{
    app::{App, View, tag_context::TagAction},
    config::Config,
    connections::{Connection, Sample},
    model::{Container, Inventory, Process, Service, Snapshot},
    tags::{self, Confidence},
};
fn process(name: &str, command: &str) -> Process {
    Process {
        pid: 42,
        start_time: 10,
        name: name.into(),
        command: command.into(),
        ..Default::default()
    }
}
fn has(tags: &[tags::Tag], key: &str) -> bool {
    tags.iter().any(|t| t.key == key)
}

#[test]
fn common_workloads_select_relevant_inspections_and_never_guess_gpu_hardware() {
    for (name, command, key, action) in [
        (
            "postgres",
            "postgres -D /data",
            "database",
            TagAction::Memory,
        ),
        (
            "elasticsearch",
            "elasticsearch",
            "search",
            TagAction::Storage,
        ),
        ("redis-server", "redis-server", "cache", TagAction::Memory),
        (
            "mosquitto",
            "mosquitto",
            "middleware",
            TagAction::Connections,
        ),
        (
            "fastdds",
            "fastdds discovery",
            "dds",
            TagAction::Environment,
        ),
        ("zenohd", "zenohd", "zenoh", TagAction::Environment),
        ("rviz2", "rviz2", "robotics", TagAction::Runtime),
        (
            "python3",
            "python3 -m jupyterlab",
            "notebook",
            TagAction::Files,
        ),
        (
            "python3",
            "python3 -m debugpy --listen 9000 app.py",
            "debugger",
            TagAction::Runtime,
        ),
        (
            "java",
            "java -cp server.jar kafka.Kafka server.properties",
            "middleware",
            TagAction::Connections,
        ),
        ("go", "go test ./...", "testing", TagAction::Cpu),
        (
            "rust-analyzer",
            "rust-analyzer",
            "language-server",
            TagAction::Memory,
        ),
        ("ruff", "ruff check .", "lint", TagAction::Files),
        (
            "git",
            "git fetch origin",
            "version-control",
            TagAction::NetworkSettings,
        ),
        ("uv", "uv sync", "packages", TagAction::Storage),
        (
            "gitlab-runner",
            "gitlab-runner run",
            "ci",
            TagAction::Runtime,
        ),
        (
            "terraform",
            "terraform plan",
            "automation",
            TagAction::Files,
        ),
        ("aws", "aws s3 ls", "cloud-cli", TagAction::NetworkSettings),
        (
            "celery",
            "celery -A jobs worker",
            "job-worker",
            TagAction::Cpu,
        ),
        (
            "ffmpeg",
            "ffmpeg -i input.mp4 output.mkv",
            "media-convert",
            TagAction::Storage,
        ),
        (
            "blender",
            "blender scene.blend",
            "render",
            TagAction::Memory,
        ),
        ("mpv", "mpv video.mp4", "media-player", TagAction::Files),
        ("obs", "obs", "streaming", TagAction::Connections),
        (
            "ssh",
            "ssh remote.example",
            "remote",
            TagAction::NetworkSettings,
        ),
        (
            "tailscaled",
            "tailscaled",
            "vpn",
            TagAction::NetworkSettings,
        ),
        ("mihomo", "mihomo", "proxy", TagAction::Connections),
        ("unbound", "unbound", "dns", TagAction::NetworkSettings),
        ("rsync", "rsync -a source dest", "sync", TagAction::Storage),
        (
            "restic",
            "restic backup /data",
            "backup",
            TagAction::Storage,
        ),
        ("smbd", "smbd", "file-sharing", TagAction::Storage),
        ("zstd", "zstd archive", "archive", TagAction::Cpu),
        (
            "dockerd",
            "dockerd",
            "container-runtime",
            TagAction::Storage,
        ),
        (
            "kubelet",
            "kubelet",
            "orchestration",
            TagAction::Connections,
        ),
        (
            "qemu-system-x86_64",
            "qemu-system-x86_64 disk.img",
            "virtualization",
            TagAction::Memory,
        ),
        (
            "otelcol",
            "otelcol",
            "observability",
            TagAction::Connections,
        ),
        ("firefox", "firefox", "browser", TagAction::Memory),
        (
            "slack",
            "slack",
            "communication",
            TagAction::NetworkSettings,
        ),
        (
            "soffice.bin",
            "soffice.bin file.odt",
            "office",
            TagAction::Files,
        ),
        ("steam", "steam", "games", TagAction::Memory),
        ("pipewire", "pipewire", "desktop", TagAction::Runtime),
        ("tmux", "tmux", "terminal", TagAction::Files),
        ("crond", "crond", "scheduler", TagAction::Cpu),
        ("baloo_file", "baloo_file", "indexer", TagAction::Storage),
        ("clamscan", "clamscan /data", "security", TagAction::Storage),
    ] {
        let mut app = App::new(Config::default());
        app.ingest(Snapshot {
            processes: vec![process(name, command)],
            ..Default::default()
        });
        app.set_view(View::Processes);
        app.remember_selection();
        let context = app.tag_context().unwrap();
        assert!(has(&context.tags, key), "{name}: {key}");
        if cfg!(target_os = "linux")
            || !matches!(
                action,
                TagAction::Memory
                    | TagAction::Cpu
                    | TagAction::Files
                    | TagAction::Runtime
                    | TagAction::Environment
            )
        {
            assert!(
                context.actions().iter().any(|(_, _, a)| *a == action),
                "{name}: {action:?}"
            );
        }
        assert!(
            !context
                .actions()
                .iter()
                .any(|(_, _, a)| *a == TagAction::Gpu)
        );
        let keys: Vec<_> = context.actions().iter().map(|(_, k, _)| *k).collect();
        for (i, key) in keys.iter().enumerate() {
            assert!(!keys[i + 1..].contains(key));
        }
    }
}

#[test]
fn explicit_intent_and_current_resource_state_do_not_leak_from_incidental_words() {
    for (name, command) in [
        (
            "private",
            "private --name nginx --file backup.py --service redis",
        ),
        ("python3", "python3 -c 'import pytest'"),
        ("node", "node --eval ffmpeg"),
        ("java", "java -jar private.jar --name kafka.Kafka"),
    ] {
        assert!(
            tags::process(&process(name, command)).is_empty(),
            "{command}"
        );
    }
    for (name, command, key) in [
        ("cargo", "cargo test", "testing"),
        ("cargo", "cargo build", "build"),
        ("npm", "npm ci", "packages"),
        ("node", "node --test", "testing"),
        ("ollama", "ollama serve", "model-server"),
        ("vllm", "vllm serve model", "model-server"),
    ] {
        assert!(has(&tags::process(&process(name, command)), key));
    }
    assert!(has(
        &tags::process(&process("cargo", "cargo +nightly test")),
        "testing"
    ));
    let scripts = tags::process(&process("npm", "npm run build"));
    assert_eq!(
        scripts
            .iter()
            .find(|t| t.key == "build")
            .unwrap()
            .confidence,
        Confidence::Inferred
    );
    assert!(has(
        &tags::process(&process(
            "qemu-system-x86",
            "/usr/bin/qemu-system-x86_64 disk.img"
        )),
        "virtualization"
    ));
    assert!(!has(
        &tags::process(&process("ollama", "ollama list")),
        "model-server"
    ));
    assert!(!has(
        &tags::process(&process("uv", "uv run app.py --name train.py")),
        "training"
    ));
    let mut p = process("private", "private");
    p.memory = 2 * 1024 * 1024 * 1024;
    p.state = "Zombie".into();
    let tags = tags::process(&p);
    assert!(has(&tags, "memory-heavy") && has(&tags, "zombie"));
    p.memory = 0;
    p.state = "Sleep".into();
    assert!(tags::process(&p).is_empty());
}

fn fixture() -> Snapshot {
    let mut p = process(
        "python3",
        &format!(
            "python3 /jobs/train.py --description={} --tail=COMMAND_END",
            "数据👩‍💻e\u{301}".repeat(75)
        ),
    );
    p.cpu = 140.;
    p.memory = 2 * 1024 * 1024 * 1024;
    p.gpu_percent = Some(50);
    p.gpu_memory = Some(4096);
    p.read_per_sec = 12. * 1024. * 1024.;
    Snapshot {
        timestamp: 100,
        processes: vec![p],
        inventory: Inventory {
            updated_at: 100,
            runtimes: vec!["docker".into()],
            service_managers: vec!["systemd/system".into()],
            services: vec![Service {
                name: "really-long-service-name-for-tests.service".into(),
                scope: "user".into(),
                pid: Some(42),
                active: "failed".into(),
                description: "description SERVICE_END".into(),
                ..Default::default()
            }],
            containers: vec![Container {
                id: "owned-container-id".into(),
                name: "really-long-container-name-for-tests".into(),
                runtime: "docker".into(),
                image: "postgres:17".into(),
                command: format!("postgres {} CONTAINER_END", "long-argument ".repeat(70)),
                state: "restarting".into(),
                status: "unhealthy".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
        connections: Sample {
            owners_sampled_at: 100,
            sampled_at: 100,
            connections: vec![Connection {
                id: "connection".into(),
                pids: vec![42],
                process_names: vec!["python3".into()],
                protocol: "tcp".into(),
                state: "LISTEN".into(),
                local: "[::]:8123".into(),
                remote: "[::]:0".into(),
                receive_queue: Some(1024 * 1024),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn screen(terminal: &Terminal<TestBackend>, rect: ratatui::layout::Rect) -> String {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn trailing_badges_are_quiet_bounded_and_all_selection_content_can_be_read_after_resize() {
    for (w, h) in [
        (60, 18),
        (80, 24),
        (100, 30),
        (120, 30),
        (160, 48),
        (300, 70),
    ] {
        for (view, end) in [
            (View::Processes, "COMMAND_END"),
            (View::Network, "COMMAND_END"),
            (View::Services, "COMMAND_END"),
            (View::Containers, "CONTAINER_END"),
        ] {
            let mut app = App::new(Config::default());
            app.ingest(fixture());
            app.set_view(view);
            app.remember_selection();
            assert!(app.open_selection());
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            assert!(app.process_detail_rect.height > 0, "{view:?} {w}x{h}");
            if w >= 120 {
                assert!(
                    app.process_detail_rect.x >= app.table_rect.right(),
                    "{view:?} {w}x{h}"
                );
            }
            for _ in 0..app.selection_lines {
                if !app.label_hits.is_empty() {
                    break;
                }
                app.scroll_selection(1);
                terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            }
            assert!(
                !app.label_hits.is_empty(),
                "{view:?} {w}x{h}: no visible detail badges"
            );
            for hit in &app.label_hits {
                assert!(app.process_detail_rect.contains((hit.x, hit.y).into()));
                assert!(!app.table_rect.contains((hit.x, hit.y).into()));
                let cell = &terminal.backend().buffer()[(hit.x, hit.y)];
                assert!(
                    !cell
                        .modifier
                        .intersects(Modifier::BOLD | Modifier::REVERSED)
                );
                assert_ne!(cell.bg, Color::Reset);
            }
            app.selection_scroll = 0;
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            let mut read = String::new();
            // A line at a time detects losses across wraps, with every scroll position bounded.
            for _ in 0..app.selection_lines + 1 {
                read.push_str(&screen(&terminal, app.process_detail_rect));
                app.scroll_selection(1);
                terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            }
            assert!(read.contains(end), "{view:?} {w}x{h}: command tail missing");
            if view == View::Processes {
                assert!(read.contains("PID: 42"));
                assert!(read.contains("CPU: 140.0"));
                assert!(!read.contains("MEMORY: —"));
            }
            assert!(read.contains("LABELS"));
            assert!(read.contains("ALL SAMPLED FIELDS".split_at(10).0));
            assert!(
                app.selection_scroll <= app.selection_lines.saturating_sub(app.selection_viewport)
            );
        }
    }
}

#[test]
fn service_image_hints_and_sampled_failures_have_distinct_evidence() {
    let mut s = fixture();
    tags::annotate(&mut s);
    let c = &s.inventory.containers[0].tags;
    assert_eq!(
        c.iter().find(|t| t.key == "database").unwrap().confidence,
        Confidence::Inferred
    );
    for key in ["restarting", "unhealthy"] {
        assert_eq!(
            c.iter().find(|t| t.key == key).unwrap().confidence,
            Confidence::Observed
        );
    }
    assert!(has(&s.inventory.services[0].tags, "failed"));
    assert!(has(&s.connections.connections[0].tags, "socket-queue"));
    assert!(!has(&s.connections.connections[0].tags, "memory-heavy"));
}

#[test]
fn selected_process_text_and_charts_share_the_right_pane_equally() {
    for (width, height) in [(160, 30), (160, 48), (220, 70)] {
        let mut app = App::new(Config::default());
        app.ingest(fixture());
        app.set_view(View::Processes);
        app.remember_selection();
        assert!(app.open_selection());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        let rect = app.process_detail_rect;
        let graph = (rect.y..rect.bottom())
            .find(|y| {
                (rect.x..rect.right())
                    .map(|x| terminal.backend().buffer()[(x, *y)].symbol())
                    .collect::<String>()
                    .contains("Since selection")
            })
            .unwrap();
        let text_height = graph - rect.y;
        let graph_height = rect.bottom() - graph;
        assert!(
            text_height.abs_diff(graph_height) <= 1,
            "{width}x{height}: text {text_height}, charts {graph_height}"
        );
        assert_eq!(
            app.selection_viewport,
            text_height.saturating_sub(2) as usize
        );
    }
}
