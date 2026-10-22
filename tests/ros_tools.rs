use rstop::{
    app::{App, View, tag_context::TagAction},
    config::Config,
    model::{Process, Snapshot},
    ros_tools::{ListKind, Query, command_spec, parse_entries},
    tags,
};

#[test]
fn ros_actions_follow_ros_evidence_and_are_absent_from_generic_simulators_and_replays() {
    for (name, command, recognized) in [
        ("rviz2", "rviz2", true),
        (
            "python3",
            "python3 /opt/ros/humble/bin/ros2 topic list",
            true,
        ),
        (
            "custom_node",
            "custom_node --ros-args -r __node:=robot",
            true,
        ),
        ("talker", "/opt/ros/humble/lib/demo_nodes_cpp/talker", true),
        ("python3", "python3 my_node.py __name:=my_node", true),
        (
            "component_conta",
            "/opt/ros/humble/lib/rclcpp_components/component_container_mt",
            true,
        ),
        ("gazebo", "gazebo", false),
        ("ddsperf", "ddsperf", false),
        ("zenohd", "zenohd", false),
        ("echo", "echo --ros-args", true),
        (
            "cat",
            "cat /opt/ros/humble/lib/demo_nodes_cpp/talker",
            false,
        ),
        ("python3", "python3 -c print('ros2')", false),
    ] {
        let mut app = App::new(Config::default());
        app.set_view(View::Processes);
        app.ingest(Snapshot {
            processes: vec![Process {
                pid: 42,
                start_time: 7,
                name: name.into(),
                command: command.into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        app.remember_selection();
        let p = &app.snapshot.processes[0];
        assert_eq!(tags::matches(&p.tags, "ros"), recognized, "{command}");
        let context = app.tag_context().unwrap();
        assert_eq!(
            context
                .actions()
                .iter()
                .any(|(_, _, a)| *a == TagAction::Ros),
            recognized
        );
        app.replay = true;
        assert!(
            !app.tag_context()
                .unwrap()
                .actions()
                .iter()
                .any(|(_, _, a)| *a == TagAction::Ros)
        );
    }
}

#[test]
fn ros_commands_use_version_specific_cli_and_direct_discovery() {
    let queries = vec![
        Query::List(ListKind::Topics),
        Query::List(ListKind::Nodes),
        Query::List(ListKind::Services),
        Query::List(ListKind::Parameters("/robot".into())),
        Query::TopicInfo("/scan".into()),
        Query::NodeInfo("/robot".into()),
        Query::ServiceInfo("/robot/get_parameters".into()),
        Query::Echo("/scan".into(), true),
        Query::Echo("/scan".into(), false),
        Query::Hz("/scan".into()),
        Query::Parameter("/robot".into(), "use_sim_time".into()),
    ];
    for query in queries {
        let ros2 = command_spec(2, &query).unwrap();
        assert_eq!(ros2.program, "ros2");
        assert_eq!(
            ros2.args.iter().any(|a| a == "--no-daemon"),
            !matches!(query, Query::Hz(_))
        );
        assert!(ros2.timeout.as_secs() <= 8);
        let ros1 = command_spec(1, &query).unwrap();
        assert_ne!(ros1.program, "ros2");
        assert!(!ros1.args.iter().any(|a| a == "--no-daemon"));
    }
    assert!(
        command_spec(2, &Query::Echo("/scan".into(), true))
            .unwrap()
            .args
            .contains(&"--once".into())
    );
    assert_eq!(
        command_spec(1, &Query::Echo("/scan".into(), true))
            .unwrap()
            .args,
        ["echo", "-n", "1", "/scan"]
    );
    assert!(
        command_spec(2, &Query::TopicInfo("/scan".into()))
            .unwrap()
            .args
            .contains(&"--verbose".into())
    );
    for name in [
        "--help",
        "$(touch x)",
        "/a;cat",
        "/a\nb",
        "/foo//bar",
        "/9node",
    ] {
        assert!(
            command_spec(2, &Query::TopicInfo(name.into())).is_err(),
            "{name}"
        );
    }
    assert!(command_spec(2, &Query::Parameter("/node".into(), "--hide-type".into())).is_err());
}

#[test]
fn graph_parser_deduplicates_and_keeps_full_names_and_types_without_parsing_diagnostics() {
    let rows = parse_entries(
        &ListKind::Topics,
        "WARNING: ambiguous nodes\n/scan [sensor_msgs/msg/LaserScan]\n/chatter [std_msgs/msg/String]\n/scan [sensor_msgs/msg/LaserScan]\nCLI diagnostics:\n/fake [fake_msgs/msg/Fake]\n",
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "/chatter");
    assert_eq!(rows[1].types, ["sensor_msgs/msg/LaserScan"]);
    let rows = parse_entries(
        &ListKind::Parameters("/node".into()),
        "/node:\n  use_sim_time\n  foo.bar\nWARNING this should not become a parameter\n",
    );
    assert_eq!(
        rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["foo.bar", "use_sim_time"]
    );
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires sourced ROS 2 and demo_nodes_cpp; starts only an owned test node"]
fn native_ros2_queries_use_the_selected_process_domain() {
    use rstop::ros_tools::{Reply, Session};
    use std::{
        process::{Command, Stdio},
        sync::{Arc, atomic::AtomicBool},
        time::Duration,
    };
    struct OwnedNode(std::process::Child);
    impl Drop for OwnedNode {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let prefix = std::env::var("AMENT_PREFIX_PATH")
        .unwrap()
        .split(':')
        .find(|p| {
            std::path::Path::new(p)
                .join("lib/demo_nodes_cpp/talker")
                .is_file()
        })
        .unwrap()
        .to_string();
    let name = format!("rstop_test_{}", std::process::id());
    let topic = format!("/{name}/chatter");
    let node_name = format!("/{name}");
    let mut child = OwnedNode(
        Command::new(format!("{prefix}/lib/demo_nodes_cpp/talker"))
            .env("ROS_DOMAIN_ID", "173")
            .env("ROS_LOCALHOST_ONLY", "1")
            .args([
                "--ros-args",
                "-r",
                &format!("__node:={name}"),
                "-r",
                &format!("chatter:={topic}"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    std::thread::sleep(Duration::from_millis(800));
    assert!(child.0.try_wait().unwrap().is_none(), "test talker exited");
    let mut system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(child.0.id());
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let p = Process {
        pid: child.0.id(),
        start_time: system.process(pid).unwrap().start_time(),
        name: "talker".into(),
        command: format!("{prefix}/lib/demo_nodes_cpp/talker --ros-args"),
        ..Default::default()
    };
    let session = Session::from_process(p.clone()).unwrap();
    assert_eq!(session.environment.get("ROS_DOMAIN_ID").unwrap(), "173");
    let cancel = Arc::new(AtomicBool::new(false));
    for kind in [
        ListKind::Topics,
        ListKind::Nodes,
        ListKind::Services,
        ListKind::Parameters(node_name.clone()),
    ] {
        let Reply::Entries(_, entries) = session.query(Query::List(kind.clone()), &cancel).unwrap()
        else {
            panic!("not a list")
        };
        let expected = match kind {
            ListKind::Topics => &topic,
            ListKind::Nodes => &node_name,
            ListKind::Services => &format!("{node_name}/get_parameters"),
            ListKind::Parameters(_) => "use_sim_time",
        };
        assert!(
            entries.iter().any(|e| e.name == expected),
            "missing {expected}: {entries:?}"
        );
    }
    for (query, expected) in [
        (Query::TopicInfo(topic.clone()), "Publisher count: 1"),
        (Query::NodeInfo(node_name.clone()), "Publishers:"),
        (
            Query::ServiceInfo(format!("{node_name}/get_parameters")),
            "rcl_interfaces/srv/GetParameters",
        ),
        (Query::Echo(topic.clone(), true), "Hello World:"),
        (Query::Echo(topic.clone(), false), "Hello World:"),
        (Query::Hz(topic.clone()), "average rate:"),
        (
            Query::Definition(rstop::ros_tools::Entry {
                name: topic,
                types: vec![],
            }),
            "string data",
        ),
        (Query::Parameter(node_name, "use_sim_time".into()), "False"),
    ] {
        let Reply::Report(report) = session.query(query, &cancel).unwrap() else {
            panic!("not a report")
        };
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    let stale = Process {
        start_time: p.start_time + 1,
        ..p
    };
    assert!(Session::from_process(stale).is_err());
}
