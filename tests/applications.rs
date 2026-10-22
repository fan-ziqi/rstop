use rstop::{
    applications::{category, index},
    connections::{Connection, Sample},
    model::{Container, Inventory, Process, Service, Snapshot},
};

fn process(pid: u32, name: &str, command: &str) -> Process {
    Process {
        pid,
        name: name.into(),
        command: command.into(),
        user: "alice".into(),
        start_time: 10,
        cpu: 5.,
        memory: 100,
        ..Default::default()
    }
}

#[test]
fn unknown_programs_and_distinct_interpreter_jobs_remain_findable() {
    let s = Snapshot {
        processes: vec![
            process(20, "python3", "/usr/bin/python3 /jobs/train.py --batch 32"),
            process(21, "python3", "/usr/bin/python3 /jobs/api.py"),
            process(
                22,
                "my-private-tool",
                "/home/alice/bin/my-private-tool --project robot",
            ),
            Process {
                thread: true,
                ..process(
                    23,
                    "my-private-tool",
                    "/home/alice/bin/my-private-tool --project robot",
                )
            },
        ],
        ..Default::default()
    };
    let apps = index(&s);
    assert_eq!(apps.len(), 3);
    assert_eq!(apps.iter().filter(|a| a.matches("python3")).count(), 2);
    assert_eq!(
        apps.iter().find(|a| a.matches("train.py")).unwrap().members,
        vec![(20, 10)]
    );
    let private = apps
        .iter()
        .find(|a| a.matches("private robot alice 22"))
        .unwrap();
    assert_eq!(private.category, "Program");
    assert_eq!(private.rss, Some(100));
}

#[test]
fn classification_never_uses_a_port_or_incidental_argument_as_proof() {
    for (name, purpose) in [
        ("nginx", "Web / proxy"),
        ("postgres", "Database"),
        ("redis-server", "Cache"),
        ("firefox", "Browser"),
        ("rustc", "Build / development"),
        ("ollama", "AI / compute"),
        ("zenohd", "Messaging / middleware"),
        ("qemu-system-aarch64", "Virtualization"),
        ("tailscaled", "Network / sync"),
        ("C:\\bin\\nginx.exe", "Web / proxy"),
    ] {
        assert_eq!(category(name), purpose);
    }
    let s = Snapshot {
        processes: vec![
            process(30, "cat", "cat /tmp/nginx-ros2-zenoh.log"),
            process(
                31,
                "custom-node",
                "/opt/robot/custom-node --ros-args -r __node:=control",
            ),
        ],
        ..Default::default()
    };
    let apps = index(&s);
    assert_eq!(
        apps.iter().find(|a| a.matches("30")).unwrap().category,
        "Program"
    );
    assert_eq!(
        apps.iter().find(|a| a.matches("31")).unwrap().category,
        "Robotics / simulation"
    );
}

#[test]
fn services_include_observed_descendants_and_keep_inactive_units() {
    let s = Snapshot {
        processes: vec![
            process(40, "nginx", "nginx: master process /usr/sbin/nginx"),
            Process {
                parent: Some(40),
                ..process(41, "nginx", "nginx: worker process")
            },
        ],
        inventory: Inventory {
            updated_at: 100,
            services: vec![
                Service {
                    name: "nginx.service".into(),
                    pid: Some(40),
                    active: "active".into(),
                    memory: Some(150),
                    ..Default::default()
                },
                Service {
                    name: "private-backup.service".into(),
                    active: "failed".into(),
                    description: "Nightly archive".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let apps = index(&s);
    assert_eq!(apps.len(), 2);
    let nginx = apps.iter().find(|a| a.name == "nginx.service").unwrap();
    assert_eq!(nginx.members, vec![(40, 10), (41, 10)]);
    assert_eq!(nginx.cpu, Some(10.));
    assert_eq!(nginx.rss, Some(150));
    let stopped = apps
        .iter()
        .find(|a| a.matches("nightly archive failed"))
        .unwrap();
    assert_eq!(stopped.cpu, None);
    assert!(stopped.members.is_empty());
}

#[test]
fn shared_service_hosts_and_user_sessions_do_not_absorb_every_application() {
    let s = Snapshot {
        processes: vec![
            process(50, "systemd", "/usr/lib/systemd/systemd --user"),
            Process {
                parent: Some(50),
                ..process(51, "firefox", "/usr/bin/firefox")
            },
            process(52, "svchost.exe", "svchost.exe"),
            Process {
                parent: Some(52),
                ..process(53, "private-tool", "private-tool")
            },
        ],
        inventory: Inventory {
            services: vec![
                Service {
                    name: "user@1000.service".into(),
                    pid: Some(50),
                    ..Default::default()
                },
                Service {
                    name: "shared-a".into(),
                    pid: Some(52),
                    ..Default::default()
                },
                Service {
                    name: "shared-b".into(),
                    pid: Some(52),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let apps = index(&s);
    assert_eq!(
        apps.iter().find(|a| a.name == "firefox").unwrap().members,
        vec![(51, 10)]
    );
    assert!(
        apps.iter()
            .filter(|a| a.service.is_some())
            .all(|a| !a.members.contains(&(53, 10)))
    );
}

#[test]
fn containers_require_exact_cgroup_evidence_and_shared_sockets_are_counted_once() {
    let id = "0123456789abcdef0123456789abcdef";
    let mut a = process(60, "worker", "worker");
    a.extra.insert(
        "cgroup".into(),
        serde_json::json!(format!("0::/system.slice/docker-{id}.scope")),
    );
    let mut b = process(61, "worker", "worker");
    b.extra = a.extra.clone();
    let s = Snapshot {
        processes: vec![
            a,
            b,
            process(62, "worker", &format!("worker --container {id}")),
        ],
        inventory: Inventory {
            containers: vec![Container {
                id: id.into(),
                name: "private-workload".into(),
                runtime: "docker".into(),
                memory: Some(90),
                cpu: Some(12.),
                ..Default::default()
            }],
            ..Default::default()
        },
        connections: Sample {
            owners_sampled_at: 100,
            connections: vec![Connection {
                id: "shared".into(),
                pids: vec![60, 61],
                local: "0.0.0.0:34567".into(),
                remote: "10.0.0.5:7447".into(),
                protocol: "udp".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let apps = index(&s);
    let c = apps.iter().find(|a| a.container.is_some()).unwrap();
    assert_eq!(c.members, vec![(60, 10), (61, 10)]);
    assert_eq!(c.sockets, vec![0]);
    assert_eq!(c.rss, Some(90));
    assert!(c.matches("10.0.0.5:7447 udp"));
    assert_eq!(
        apps.iter().find(|a| a.kind == "Program").unwrap().members,
        vec![(62, 10)]
    );
}

#[test]
fn stale_owner_samples_and_reused_parents_do_not_create_relationships() {
    let s = Snapshot {
        processes: vec![
            Process {
                start_time: 200,
                ..process(70, "nginx", "nginx")
            },
            Process {
                parent: Some(70),
                ..process(71, "custom", "custom")
            },
        ],
        inventory: Inventory {
            updated_at: 100,
            services: vec![Service {
                name: "nginx.service".into(),
                pid: Some(70),
                ..Default::default()
            }],
            ..Default::default()
        },
        connections: Sample {
            owners_sampled_at: 100,
            connections: vec![Connection {
                pids: vec![70],
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let apps = index(&s);
    assert!(
        apps.iter()
            .find(|a| a.service.is_some())
            .unwrap()
            .members
            .is_empty()
    );
    assert!(apps.iter().all(|a| a.sockets.is_empty()));
}

#[test]
fn cached_container_ownership_is_bound_to_process_start_time() {
    let id = "1234567890abcdef1234567890abcdef";
    let mut s = Snapshot {
        processes: vec![process(110, "private", "private")],
        inventory: Inventory {
            containers: vec![Container {
                id: id.into(),
                name: "private-container".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    s.inventory.extra.insert(
        "Application ownership".into(),
        serde_json::json!({"cgroups":{"110:10":format!("0::/docker/{id}")},"complete":true}),
    );
    assert_eq!(
        index(&s)
            .iter()
            .find(|a| a.container.is_some())
            .unwrap()
            .members,
        vec![(110, 10)]
    );
    s.processes[0].start_time = 20;
    assert!(
        index(&s)
            .iter()
            .find(|a| a.container.is_some())
            .unwrap()
            .members
            .is_empty()
    );
}

#[test]
fn arbitrary_service_launchers_keep_different_programs_and_interpreter_jobs_separate() {
    let mut root = process(120, "custom-launcher", "custom-launcher");
    root.cpu = 1.;
    let s = Snapshot {
        processes: vec![
            root,
            Process {
                parent: Some(120),
                ..process(121, "python3", "python3 -m http.server")
            },
            Process {
                parent: Some(120),
                ..process(122, "python3", "python3 train.py")
            },
        ],
        inventory: Inventory {
            services: vec![Service {
                name: "unknown-launcher.service".into(),
                pid: Some(120),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let apps = index(&s);
    assert_eq!(apps.len(), 3);
    assert_eq!(
        apps.iter()
            .find(|a| a.matches("http.server"))
            .unwrap()
            .members,
        vec![(121, 10)]
    );
    assert_eq!(
        apps.iter().find(|a| a.service.is_some()).unwrap().members,
        vec![(120, 10)]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn loaded_component_inspection_rejects_a_changed_process_identity() {
    let pid = std::process::id();
    let mut system = sysinfo::System::new();
    let id = sysinfo::Pid::from_u32(pid);
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[id]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    let mut p = process(pid, "test-owned", "test-owned");
    p.start_time = system.process(id).unwrap().start_time();
    let result = rstop::applications::inspect_members(&[p.clone()]);
    assert!(result.contains("Loaded shared objects"));
    assert!(result.contains("Executable:"));
    p.start_time += 1;
    let stale = rstop::applications::inspect_members(&[p]);
    assert!(stale.contains("PID identity changed"));
    assert!(!stale.contains("Executable:"));
}
