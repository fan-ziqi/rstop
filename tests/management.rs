use rstop::{management::*, model::*};
#[test]
fn workload_plans_bind_identity_and_reject_untrusted_arguments() {
    let service = Service {
        name: "example.service".into(),
        scope: "user".into(),
        ..Default::default()
    };
    #[cfg(any(target_os = "linux", windows, target_os = "macos"))]
    for action in service_actions() {
        if cfg!(target_os = "macos") && ["configuration", "load"].contains(action) {
            continue;
        }
        let plan = service_plan(&service, action).unwrap();
        assert!(!plan.commands.is_empty());
        assert!(plan.preview().contains("example.service") || *action == "daemon-reload");
        if plan.changes_state {
            assert!(
                plan.execute(true)
                    .unwrap_err()
                    .to_string()
                    .contains("disabled")
            );
        }
    }
    let mut bad = service.clone();
    bad.name = "--all".into();
    assert!(service_plan(&bad, "stop").is_err());
    bad.name = "good\nevil".into();
    assert!(service_plan(&bad, "stop").is_err());
    assert!(service_plan(&service, "delete-root").is_err());
    let container = Container {
        runtime: "docker".into(),
        id: "0123456789abcdef".into(),
        name: "fixture".into(),
        ..Default::default()
    };
    for action in container_actions() {
        let plan = container_plan(&container, action).unwrap();
        assert!(plan.commands[0].1.contains(&container.id));
        if plan.changes_state {
            assert!(plan.execute(true).is_err());
        }
    }
    let mut bad = container.clone();
    bad.id = "--all".into();
    assert!(container_plan(&bad, "stop").is_err());
    bad = container;
    bad.runtime = "sh".into();
    assert!(container_plan(&bad, "start").is_err());
}
#[test]
fn management_overlay_handles_all_terminal_sizes() {
    use ratatui::{Terminal, backend::TestBackend};
    let service = Service {
        name: "example.service".into(),
        scope: "user".into(),
        ..Default::default()
    };
    let plans: Vec<_> = service_actions()
        .iter()
        .filter_map(|action| {
            service_plan(&service, action)
                .ok()
                .map(|p| (action.to_string(), p))
        })
        .collect();
    if plans.is_empty() {
        return;
    }
    for (w, h) in [(60, 18), (80, 24), (160, 48)] {
        let mut app = rstop::app::App::new(Default::default());
        app.action_menu = Some((plans.clone(), plans.len() - 1));
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
        app.action_menu = None;
        app.pending_plan = Some(plans[0].1.clone());
        app.prompt = Some(("workload action".into(), "yes".into(), None));
        term.draw(|f| rstop::ui::draw(f, &mut app)).unwrap();
    }
}
#[cfg(target_os = "linux")]
#[test]
#[ignore = "Creates and controls only a dedicated transient rstop-test user service"]
fn owned_transient_service_restart_logs_and_stop() {
    use std::process::Command;
    let name = format!("rstop-test-{}.service", std::process::id());
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = Command::new("systemctl")
                .args(["--user", "stop", &self.0])
                .status();
            let _ = Command::new("systemctl")
                .args(["--user", "reset-failed", &self.0])
                .output();
        }
    }
    let _cleanup = Cleanup(name.clone());
    let result = Command::new("systemd-run")
        .args([
            "--user",
            "--quiet",
            "--unit",
            &name,
            "--",
            "/usr/bin/python3",
            "-u",
            "-c",
            "import time; print('rstop-management-test', flush=True); time.sleep(60)",
        ])
        .status()
        .unwrap();
    assert!(result.success());
    let service = Service {
        name: name.clone(),
        scope: "user".into(),
        active: "active".into(),
        ..Default::default()
    };
    let status = service_plan(&service, "status")
        .unwrap()
        .execute(true)
        .unwrap();
    assert!(status.contains("ActiveState=active"));
    service_plan(&service, "restart")
        .unwrap()
        .execute(false)
        .unwrap();
    let mut logs = String::new();
    let began = std::time::Instant::now();
    while began.elapsed() < std::time::Duration::from_secs(3) {
        logs = rstop::discovery::command(
            "journalctl",
            &["--user", "-u", &name, "-n", "30", "--no-pager"],
            std::time::Duration::from_secs(2),
        )
        .unwrap();
        if logs.contains("rstop-management-test") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(logs.contains("rstop-management-test"));
    let pid: u32 = rstop::discovery::command(
        "systemctl",
        &["--user", "show", &name, "--property=MainPID", "--value"],
        std::time::Duration::from_secs(2),
    )
    .unwrap()
    .trim()
    .parse()
    .unwrap();
    let id = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[id]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    let process = rstop::model::Process {
        pid,
        start_time: system.process(id).unwrap().start_time(),
        ..Default::default()
    };
    let output = rstop::logs::Target::process(&process, &rstop::model::Snapshot::default())
        .read(30)
        .unwrap();
    assert!(output.contains("rstop-management-test"));
    assert!(output.contains("whole unit") && output.contains(&name));
    service_plan(&service, "stop")
        .unwrap()
        .execute(false)
        .unwrap();
    assert!(
        !Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", &name])
            .status()
            .unwrap()
            .success()
    );
}
