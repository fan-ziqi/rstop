#[cfg(target_os = "linux")]
use anyhow::Result;
use rstop::{
    app::{App, View},
    discovery,
    model::*,
};
#[cfg(target_os = "linux")]
use std::fs;
use std::{sync::mpsc, thread, time::Duration};
fn process_gpu_details(app: &App, pid: u32) -> String {
    let devices: Vec<_> = app.snapshot.gpus.iter().filter_map(|device| {
        device.processes.iter().find(|p|p.pid == pid).map(|metrics| serde_json::json!({"device":device.id,"name":device.name,"process":metrics}))
    }).collect();
    rstop::fields::pretty(&serde_json::json!(devices))
}
pub(super) fn inspect(app: &mut App) {
    if app.view == View::Gpu && app.gpu_section == 2 {
        app.detail = app
            .snapshot
            .gpus
            .get(app.selected_gpu)
            .and_then(|gpu| {
                rstop::fields::gpu(gpu, &app.filter)
                    .get(app.selected)
                    .cloned()
            })
            .map(|(group, name, value)| (format!("{group} · {name}"), value));
        app.detail_scroll = 0;
        app.detail_column = 0;
        return;
    }
    if app.replay && app.has_process_table() {
        app.detail = app.selected_process().map(|p| {
            (
                "RECORDED PROCESS".into(),
                format!(
                    "{}\n\nPer-GPU process counters\n{}",
                    rstop::fields::pretty(&serde_json::to_value(p).unwrap_or_default()),
                    process_gpu_details(app, p.pid)
                ),
            )
        });
        return;
    }

    let detail=match app.view{
        View::Overview|View::Processes|View::Gpu=>app.selected_process().map(|p|{
            let mut body=format!("{}\n\nPID         {}    parent {:?}\nUser        {}\nState       {}\nCPU         {:.1}% (100% = one logical core)\nMemory      {}\nDisk read   {}/s\nDisk write  {}/s\nVRAM        {}\nGPU devices {:?}\nRuntime     {}\n\nCommand\n{}",p.name,p.pid,p.parent,p.user,p.state,p.cpu,bytes(p.memory as f64),bytes(p.read_per_sec),bytes(p.write_per_sec),p.gpu_memory.map(|v|bytes(v as f64)).unwrap_or_else(||"—".into()),p.gpu_ids,duration(p.runtime),p.command);
            #[cfg(target_os="linux")]{
                for(label,file)in[("Working directory","cwd"),("Executable","exe")]{if let Ok(path)=fs::read_link(format!("/proc/{}/{file}",p.pid)){body.push_str(&format!("\n\n{label}\n{}",path.display()));}}
                if let Ok(cgroup)=fs::read_to_string(format!("/proc/{}/cgroup",p.pid)){body.push_str(&format!("\n\nCgroups\n{cgroup}"));}
                if let Ok(entries)=fs::read_dir(format!("/proc/{}/fd",p.pid)){body.push_str(&format!("\nOpen file descriptors: {}",entries.count()));}
            }
            body.push_str(&format!("\n\nRecognized labels\n{}\nClose detail, then [F6] Tools > Labels and related actions.\n\nAll sampled fields\n{}",rstop::tags::explain(&p.tags),rstop::fields::pretty(&serde_json::to_value(p).unwrap_or_default())));
            body.push_str(&format!("\n\nPer-GPU process counters\n{}", process_gpu_details(app, p.pid)));
            #[cfg(target_os="linux")]{
                for file in ["status","statm","io","schedstat","oom_score","oom_score_adj","autogroup","wchan","smaps_rollup","limits"] {
                    match fs::read_to_string(format!("/proc/{}/{file}",p.pid)) {
                        Ok(value) => body.push_str(&format!("\n\n{file}\n{value}")),
                        Err(error) => body.push_str(&format!("\n\n{file}: {error}")),
                    }
                }
            }
            (format!("PROCESS {}",p.pid),body)
        }),
        View::Containers=>app.filtered_containers().get(app.selected).map(|c| {
            (format!("CONTAINER {}",c.name), format!("{}\n\nNetwork and block IO are cumulative totals.\nPress F6 for tools, F7 for the configured log tail.",
                rstop::fields::pretty(&serde_json::to_value(c).unwrap_or_default())))
        }),
        View::Services=>app.filtered_services().get(app.selected).map(|s| (format!("SERVICE {}",s.name),format!("{}\n\nPress F6 for tools, F7 for logs.",rstop::fields::pretty(&serde_json::to_value(s).unwrap_or_default())))),
        View::Explorer=>app.extra_rows().get(app.selected).map(|(name,value)|(name.clone(),rstop::fields::pretty(value))),
        View::History=>app.history.series.iter().filter(|(key,_)|key.to_lowercase().contains(&app.filter.to_lowercase())).nth(app.selected).map(|(name,values)|(name.clone(),format!("{values:?}"))),
        View::Events=>app.events.history.iter().rev().filter(|event|rstop::filter::event_matches(event,&app.filter)).nth(app.selected).map(|event|("Event".into(),format!("{event:?}"))),
        View::System => Some(("SYSTEM DETAILS".into(), rstop::fields::pretty(&serde_json::json!({
            "hostname":app.snapshot.hostname,"kernel":app.snapshot.kernel,"cpu_breakdown":app.snapshot.cpu_breakdown,
            "pressure":app.snapshot.pressure,"sensors":app.snapshot.sensors,"batteries":app.snapshot.batteries,
            "listeners":app.snapshot.inventory.listeners,"diagnostics":app.snapshot.inventory.notes,"alerts":app.snapshot.alerts,
        })))),
        View::Storage => Some(("STORAGE DETAILS".into(), rstop::fields::pretty(&serde_json::json!({"filesystems":app.snapshot.disks,"block_devices":app.snapshot.disk_io})))),
        View::Network if app.network_section == 0 => rstop::ui::network::detail(app),
        View::Network => app.filtered_networks().get(app.selected).map(|n|(format!("INTERFACE {}",n.name),rstop::fields::pretty(&serde_json::to_value(n).unwrap_or_default()))),
    };
    if let Some(detail) = detail {
        app.detail = Some(detail);
        app.detail_scroll = 0;
    }
}

pub(super) fn load_resource_detail(app: &mut App, sender: mpsc::Sender<(u64, String, String)>) {
    if app.replay {
        return;
    }
    if app.detail_pending {
        return;
    }
    // The service branch returns None on unsupported platforms.
    #[allow(clippy::bind_instead_of_map)]
    let request = match app.view {
        View::Containers => app.filtered_containers().get(app.selected).map(|c| (
            c.runtime.clone(), vec!["inspect".into(), "--format".into(), "{{json .}}".into(), c.id.clone()], true,
        )),
        View::Services => app.filtered_services().get(app.selected).and_then(|service| {
            #[cfg(target_os = "linux")]
            { Some(("systemctl".into(), vec![format!("--{}", service.scope), "show".into(), "--no-pager".into(), "--property=Id,Description,MainPID,ControlGroup,ActiveEnterTimestamp,ExecMainStatus,NRestarts,MemoryCurrent,TasksCurrent,CPUUsageNSec,FragmentPath,UnitFileState,Requires,Wants,After,Restart".into(), "--".into(), service.name.clone()], false)) }
            #[cfg(target_os = "macos")]
            { Some(("launchctl".into(), vec!["print".into(), format!("gui/{}/{}", unsafe { libc::geteuid() }, service.name)], false)) }
            #[cfg(target_os = "windows")]
            { Some(("sc.exe".into(), vec!["queryex".into(), service.name.clone()], false)) }
            #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
            { let _ = service; None }
        }),
        _ => None,
    };
    let Some((program, args, container)) = request else {
        return;
    };
    let Some((title, initial)) = app.detail.clone() else {
        return;
    };
    app.detail_pending = true;
    app.detail_generation += 1;
    let generation = app.detail_generation;
    thread::spawn(move || {
        let refs: Vec<_> = args.iter().map(String::as_str).collect();
        let extra = match discovery::command(&program, &refs, Duration::from_secs(4)) {
            Ok(output) if container => match serde_json::from_str::<serde_json::Value>(&output) {
                Ok(value) => {
                    let selected = serde_json::json!({
                        "HostPID": value["State"]["Pid"],
                        "Health": value["State"]["Health"]["Status"],
                        "ExitCode": value["State"]["ExitCode"],
                        "OOMKilled": value["State"]["OOMKilled"],
                        "Error": value["State"]["Error"],
                        "FinishedAt": value["State"]["FinishedAt"],
                        "HealthChecks": value["State"]["Health"]["Log"],
                        "RestartCount": value["RestartCount"],
                        "Entrypoint": value["Config"]["Entrypoint"],
                        "Command": value["Config"]["Cmd"],
                        "Mounts": value["Mounts"],
                        "Networks": value["NetworkSettings"]["Networks"],
                        "PublishedPorts": value["NetworkSettings"]["Ports"],
                        "MemoryLimit": value["HostConfig"]["Memory"],
                        "NanoCpus": value["HostConfig"]["NanoCpus"],
                        "RestartPolicy": value["HostConfig"]["RestartPolicy"],
                    });
                    rstop::fields::pretty(&selected)
                }
                Err(error) => format!("Invalid inspect response: {error}"),
            },
            Ok(output) => output,
            Err(error) => format!("Cannot inspect live resource: {error}"),
        };
        let _ = sender.send((
            generation,
            title,
            format!("{initial}\n\nLive inspection\n{extra}"),
        ));
    });
}

pub(super) fn load_logs(app: &mut App, sender: mpsc::Sender<(u64, String, String)>) {
    use rstop::logs::Target;
    if app.replay {
        app.status = "Logs are not embedded in snapshot recordings".into();
        return;
    }
    if app.detail_pending {
        return;
    }
    let opening = app.log_target.is_none();
    let request = app.log_target.clone().or_else(|| {
        if app.has_process_table() {
            app.selected_process()
                .map(|p| Target::process(p, &app.snapshot))
        } else {
            match app.view {
                View::Containers => app
                    .filtered_containers()
                    .get(app.selected)
                    .map(|c| Target::container(c, app.config.log_lines)),
                View::Services => app
                    .filtered_services()
                    .get(app.selected)
                    .and_then(|s| Target::service(s, app.config.log_lines)),
                _ => None,
            }
        }
    });
    let Some(target) = request else {
        app.status = "Select a process, container or service for output / logs".into();
        return;
    };
    let title = target.title();
    if opening {
        app.follow_logs = true;
        app.detail_scroll = 0;
        app.detail_column = 0;
        app.detail_filter.clear();
        app.detail = Some((title.clone(), "Finding output sources…".into()));
    }
    app.log_target = Some(target.clone());
    app.detail_pending = true;
    app.detail_generation += 1;
    let generation = app.detail_generation;
    let lines = app.config.log_lines;
    thread::spawn(move || {
        let body = target.read(lines).unwrap_or_else(|error| {
            format!("Cannot read output: {error}\n\nPress p to follow a log file.")
        });
        let _ = sender.send((generation, title, body));
    });
}

pub(super) fn inspect_process_extra(
    app: &mut App,
    environment: bool,
    sender: mpsc::Sender<(u64, String, String)>,
) {
    if app.replay {
        app.status = "Live process inspection is disabled during replay".into();
        return;
    }
    if app.detail_pending {
        return;
    }
    if !app.has_process_table() {
        return;
    }
    let Some(process) = app.selected_process() else {
        return;
    };
    let pid = process.pid;
    app.detail_pending = true;
    app.detail_generation += 1;
    let generation = app.detail_generation;
    let title = format!(
        "{} · PID {pid}",
        if environment {
            "ENVIRONMENT"
        } else {
            "OPEN FILES"
        }
    );
    app.status = format!("Loading {title}…");
    app.detail = Some((title.clone(), "Loading…".into()));
    thread::spawn(move || {
        #[cfg(target_os = "linux")]
        let result = (|| -> Result<String> {
            if environment {
                let data = fs::read(format!("/proc/{pid}/environ"))?;
                Ok(String::from_utf8_lossy(&data)
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n"))
            } else {
                let mut files = vec![];
                for entry in fs::read_dir(format!("/proc/{pid}/fd"))?.flatten() {
                    files.push(format!(
                        "{:>5}  {}",
                        entry.file_name().to_string_lossy(),
                        fs::read_link(entry.path())
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|e| format!("unreadable: {e}"))
                    ));
                }
                files.sort();
                Ok(files.join("\n"))
            }
        })();
        #[cfg(target_os = "linux")]
        let body = result.unwrap_or_else(|error| format!("Cannot inspect process: {error}"));
        #[cfg(target_os = "macos")]
        let body = if environment {
            discovery::command(
                "ps",
                &["eww", "-p", &pid.to_string(), "-o", "command="],
                Duration::from_secs(2),
            )
        } else {
            discovery::command(
                "lsof",
                &["-nP", "-p", &pid.to_string()],
                Duration::from_secs(3),
            )
        }
        .unwrap_or_else(|e| e.to_string());
        #[cfg(windows)]
        let body = if environment {
            rstop::windows_inspect::environment(pid)
        } else {
            discovery::command(
                "handle.exe",
                &["-nobanner", "-p", &pid.to_string()],
                Duration::from_secs(3),
            )
        }
        .unwrap_or_else(|e| e.to_string());
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        let body = "Process inspection backend unavailable".to_string();
        let _ = sender.send((generation, title, body));
    });
}

pub(super) fn open_report(app: &mut App, title: &str, body: String) {
    app.detail_generation += 1;
    app.detail_pending = false;
    app.log_target = None;
    app.follow_logs = false;
    app.detail_filter.clear();
    app.detail_search = false;
    app.detail_scroll = 0;
    app.detail_column = 0;
    app.detail = Some((title.into(), body));
}
pub(super) fn task(
    app: &mut App,
    title: &str,
    sender: mpsc::Sender<(u64, String, String)>,
    work: impl FnOnce() -> anyhow::Result<String> + Send + 'static,
) {
    open_report(app, title, "Collecting evidence… · [Esc] Back".into());
    app.detail_pending = true;
    let generation = app.detail_generation;
    let title = title.to_string();
    thread::spawn(move || {
        let body = work().unwrap_or_else(|e| format!("Cannot complete check: {e}"));
        let _ = sender.send((generation, title, body));
    });
}
pub(super) fn additional(app: &mut App, sender: mpsc::Sender<(u64, String, String)>) {
    let mut body = rstop::diagnostics::sampled_details(app);
    if app.replay {
        body.push_str("\nRecorded data only; live checks disabled during replay.");
        open_report(app, "RESOURCE DETAILS", body);
        return;
    }
    match app.view {
        View::Network => task(app, "ROUTES / DNS", sender, move || {
            Ok(format!("{}\n{body}", rstop::network_tools::settings()))
        }),
        View::Storage => {
            let mut frozen = App::new(app.config.clone());
            frozen.snapshot = app.snapshot.clone();
            task(app, "STORAGE CHECKS", sender, move || {
                Ok(format!(
                    "{}\n{body}",
                    rstop::diagnostics::storage_checks(&frozen)
                ))
            });
        }
        _ => open_report(app, "RESOURCE DETAILS", body),
    }
}
