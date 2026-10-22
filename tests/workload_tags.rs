use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::{App, View, tag_context::TagAction},
    config::Config,
    connections::{Connection, Sample},
    model::{Container, Gpu, Inventory, Process, Service, Snapshot},
    tags::{self, Confidence},
};

fn process(pid: u32, name: &str, command: &str) -> Process {
    Process {
        pid,
        start_time: 10,
        name: name.into(),
        command: command.into(),
        user: "test".into(),
        ..Default::default()
    }
}
fn has(p: &Process, key: &str) -> bool {
    p.tags.iter().any(|t| t.key == key)
}
fn fixture() -> Snapshot {
    Snapshot {
        timestamp: 100,
        processes: vec![
            process(42, "python3", "python3 -m http.server 8080"),
            process(43, "private", "private --port 5432"),
        ],
        inventory: Inventory {
            updated_at: 100,
            runtimes: vec!["docker".into()],
            service_managers: vec!["systemd/system".into()],
            services: vec![Service {
                name: "private-web.service".into(),
                pid: Some(42),
                active: "active".into(),
                ..Default::default()
            }],
            containers: vec![Container {
                id: "abcdef0123456789abcdef".into(),
                name: "private-db".into(),
                image: "registry.example/postgres:17".into(),
                state: "running".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
        connections: Sample {
            sampled_at: 100,
            owners_sampled_at: 100,
            connections: vec![
                Connection {
                    id: "listener".into(),
                    pids: vec![42],
                    process_names: vec!["python3".into()],
                    protocol: "tcp".into(),
                    local: "0.0.0.0:8080".into(),
                    remote: "0.0.0.0:0".into(),
                    state: "LISTEN".into(),
                    ..Default::default()
                },
                Connection {
                    id: "private".into(),
                    pids: vec![43],
                    protocol: "tcp".into(),
                    local: "127.0.0.1:5432".into(),
                    remote: "192.0.2.1:443".into(),
                    state: "ESTABLISHED".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn inferred_intent_is_marked_and_unrelated_arguments_never_become_purpose() {
    let mut training = process(1, "python3", "python3 -u /jobs/train.py --epochs 100");
    training.tags = tags::process(&training);
    let tag = training.tags.iter().find(|t| t.key == "training").unwrap();
    assert_eq!(tag.confidence, Confidence::Inferred);
    assert_eq!(tag.display(), "Training?");
    assert!(tag.evidence[0].contains("train.py"));
    for mut p in [
        process(2, "cat", "cat train.py nginx.log"),
        process(3, "private", "private --file train.py"),
        process(4, "python3", "python3 -c print_train.py"),
        process(5, "python3", "python3 --version"),
    ] {
        p.tags = tags::process(&p);
        assert!(!has(&p, "training"));
        assert!(!has(&p, "web"));
    }
}

#[test]
fn gpu_or_distributed_launch_does_not_establish_training_and_stale_resource_tags_expire() {
    let mut p = process(20, "torchrun", "torchrun inference.py");
    p.gpu_percent = Some(90);
    p.gpu_memory = Some(1024);
    p.gpu_ids = vec![0];
    p.tags = tags::process(&p);
    assert!(has(&p, "distributed-compute"));
    assert!(has(&p, "gpu-active"));
    assert!(!has(&p, "training"));
    p.gpu_percent = None;
    p.tags = tags::process(&p);
    assert!(has(&p, "gpu-memory"));
    assert!(!has(&p, "gpu-active"));
    p.gpu_memory = None;
    p.gpu_ids.clear();
    p.tags = tags::process(&p);
    assert!(!has(&p, "gpu-memory"));
}

#[test]
fn socket_labels_need_owner_evidence_and_never_guess_the_service_from_its_port() {
    let mut s = fixture();
    tags::annotate(&mut s);
    assert!(tags::matches(&s.connections.connections[0].tags, "web"));
    assert!(!tags::matches(
        &s.connections.connections[1].tags,
        "database"
    ));
    assert!(!tags::matches(&s.connections.connections[1].tags, "web"));
    s.processes[0].start_time = 200;
    tags::annotate(&mut s);
    assert!(!tags::matches(&s.connections.connections[0].tags, "web"));
    assert!(tags::matches(
        &s.connections.connections[0].tags,
        "listener"
    ));
}

#[test]
fn ownership_propagates_roles_but_not_unrelated_resource_activity() {
    let mut s = fixture();
    s.processes[0].gpu_percent = Some(80);
    s.processes[0].cpu = 200.;
    tags::annotate(&mut s);
    assert!(tags::matches(&s.inventory.services[0].tags, "web"));
    assert!(tags::matches(&s.inventory.services[0].tags, "gpu-active"));
    assert!(!tags::matches(
        &s.connections.connections[0].tags,
        "gpu-active"
    ));
    let image = s.inventory.containers[0]
        .tags
        .iter()
        .find(|t| t.key == "database")
        .unwrap();
    assert_eq!(image.confidence, Confidence::Inferred);
    assert!(!tags::matches(&s.processes[1].tags, "database"));
    let once = serde_json::to_value(&s.processes[0].tags).unwrap();
    tags::annotate(&mut s);
    assert_eq!(once, serde_json::to_value(&s.processes[0].tags).unwrap());
}

#[test]
fn filters_find_labels_in_process_network_service_and_container_pages() {
    let mut app = App::new(Config::default());
    app.ingest(fixture());
    for (view, query, count) in [
        (View::Processes, "tag:web", 1),
        (View::Network, "tag:web state:LISTEN", 1),
        (View::Services, "tag:web", 1),
        (View::Containers, "tag:database", 1),
    ] {
        app.set_view(view);
        app.filter = query.into();
        assert_eq!(app.row_count(), count, "{view:?}");
    }
    app.set_view(View::Processes);
    app.filter = "tag:web | name:private".into();
    assert_eq!(app.row_count(), 2);
    app.filter = "!tag:web".into();
    assert_eq!(app.row_count(), 1);
}

#[test]
fn label_actions_vary_by_role_actual_devices_and_replay_capabilities() {
    let mut app = App::new(Config::default());
    app.ingest(fixture());
    app.set_view(View::Processes);
    app.filter = "pid:42".into();
    app.remember_selection();
    let ctx = app.tag_context().unwrap();
    let actions = ctx.actions();
    assert!(actions.iter().any(|(_, _, a)| *a == TagAction::Test));
    assert_eq!(ctx.endpoint.as_deref(), Some("127.0.0.1:8080"));
    assert!(!actions.iter().any(|(_, _, a)| *a == TagAction::Gpu));
    app.replay = true;
    let actions = app.tag_context().unwrap().actions();
    assert!(actions.iter().all(|(_, _, a)| !matches!(
        a,
        TagAction::Logs
            | TagAction::Test
            | TagAction::Environment
            | TagAction::Files
            | TagAction::Runtime
            | TagAction::Memory
            | TagAction::Cpu
            | TagAction::NetworkSettings
    )));
    app.replay = false;
    let mut s = fixture();
    s.processes[0] = process(42, "custom-robot", "custom-robot --ros-args");
    s.processes[0].gpu_ids = vec![7];
    s.gpus = vec![Gpu {
        id: 7,
        ..Default::default()
    }];
    app.ingest(s);
    let ctx = app.tag_context().unwrap();
    assert!(ctx.actions().iter().any(|(_, _, a)| *a == TagAction::Gpu));
    if cfg!(target_os = "linux") {
        assert!(
            ctx.actions()
                .iter()
                .any(|(_, _, a)| *a == TagAction::Environment)
        );
    }
}

#[test]
fn frozen_related_scope_never_adopts_a_reused_process_id() {
    let mut app = App::new(Config::default());
    let mut s = fixture();
    app.ingest(s.clone());
    app.set_view(View::Processes);
    app.filter = "pid:42".into();
    app.remember_selection();
    let ctx = app.tag_context().unwrap();
    app.open_tag_scope(&ctx, false);
    assert_eq!(app.process_rows().len(), 1);
    s.processes[0].start_time = 200;
    app.ingest(s);
    assert!(app.process_rows().is_empty());
    app.open_tag_scope(&ctx, true);
    assert_eq!(app.filtered_connections().len(), 1);
    assert_eq!(app.filtered_connections()[0].id, "listener");
    app.set_view(View::Network);
    assert_eq!(app.filtered_connections().len(), 2);
}

#[test]
fn snapshots_and_legacy_replay_keep_explainable_labels_without_a_separate_page() {
    let old: Snapshot = serde_json::from_str(
        r#"{"processes":[{"pid":42,"start_time":10,"name":"nginx","command":"nginx"}]}"#,
    )
    .unwrap();
    let mut app = App::new(Config::default());
    app.replay = true;
    app.ingest(old);
    assert!(has(&app.snapshot.processes[0], "web"));
    let saved = serde_json::to_string(&app.snapshot).unwrap();
    let restored: Snapshot = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.processes[0].tags, app.snapshot.processes[0].tags);
    assert!(app.views().iter().all(|v| v.name() != "Applications"));
    assert!(
        app.views()
            .iter()
            .all(|v| matches!(v.key(), crossterm::event::KeyCode::Char('1'..='8')))
    );
}

#[test]
fn inline_badges_and_label_tools_remain_available_on_existing_pages_at_all_sizes() {
    for (w, h) in [(60, 18), (100, 30), (160, 48)] {
        let mut app = App::new(Config::default());
        app.ingest(fixture());
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        for (view, query, identity) in [
            (View::Processes, "tag:web", "python3"),
            (View::Network, "tag:web", "unknown"),
            (View::Services, "tag:web", "private-web"),
            (View::Containers, "tag:database", "private-db"),
        ] {
            app.set_view(view);
            app.filter = query.into();
            app.remember_selection();
            assert!(app.open_selection());
            terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(text.contains(identity), "{view:?} {w}x{h}");
            assert!(text.contains("[F6] Tools"));
            assert!(
                app.tools_commands()
                    .iter()
                    .any(|(_, key)| *key == crossterm::event::KeyCode::Char('Y')),
                "{view:?} {w}x{h}: missing Labels tool"
            );
            for _ in 0..app.selection_lines {
                if !app.label_hits.is_empty() {
                    break;
                }
                app.scroll_selection(1);
                terminal.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
            }
            assert!(
                !app.label_hits.is_empty(),
                "{view:?} {w}x{h}: no badge hits"
            );
            let buffer = terminal.backend().buffer();
            for hit in &app.label_hits {
                assert!(app.process_detail_rect.contains((hit.x, hit.y).into()));
                assert!(!app.table_rect.contains((hit.x, hit.y).into()));
                let badge = (hit.x..hit.right())
                    .map(|x| buffer[(x, hit.y)].symbol())
                    .collect::<String>();
                assert!(!badge.trim().is_empty());
                assert!(!badge.contains('[') && !badge.contains(']'), "{badge}");
                assert!(buffer[(hit.x, hit.y)].bg != ratatui::style::Color::Reset);
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn communication_environment_is_explicit_bounded_and_identity_checked() {
    let mut child = std::process::Command::new("sleep")
        .arg("5")
        .env("ROS_DOMAIN_ID", "17")
        .env("RSTOP_PRIVATE_TEST_SECRET", "not-for-labels")
        .spawn()
        .unwrap();
    let result = std::panic::catch_unwind(|| {
        let id = sysinfo::Pid::from_u32(child.id());
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[id]),
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        let mut p = process(child.id(), "sleep", "sleep 5");
        p.start_time = system.process(id).unwrap().start_time();
        assert!(tags::process(&p).is_empty());
        let text = tags::inspect(&p, true).unwrap();
        assert!(text.contains("ROS_DOMAIN_ID=17"));
        assert!(!text.contains("not-for-labels"));
        let memory = tags::inspect_resources(&p, true).unwrap();
        assert!(memory.contains("smaps_rollup") && memory.contains("limits"));
        assert!(!memory.contains("not-for-labels"));
        let scheduling = tags::inspect_resources(&p, false).unwrap();
        assert!(scheduling.contains("schedstat") && scheduling.contains("Cpus_allowed"));
        p.start_time += 1;
        assert!(tags::inspect(&p, true).is_err());
    });
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap();
}
