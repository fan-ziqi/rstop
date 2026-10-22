//! Explicit, reviewable local workload actions. No shell is used.
use crate::{
    discovery,
    model::{Container, Service},
};
use anyhow::{Result, ensure};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Plan {
    pub title: String,
    pub commands: Vec<(String, Vec<String>)>,
    pub changes_state: bool,
}
impl Plan {
    pub fn preview(&self) -> String {
        format!(
            "{}\n\n{}\n\n{}",
            self.title,
            self.commands
                .iter()
                .map(|(p, a)| format!(
                    "{p} {}",
                    a.iter()
                        .map(|s| format!("{s:?}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            if self.changes_state {
                "[y] Confirm · [Esc] Cancel. Existing permissions apply; rstop never invokes sudo."
            } else {
                "Enter to inspect."
            }
        )
    }
    pub fn execute(&self, read_only: bool) -> Result<String> {
        ensure!(
            !read_only || !self.changes_state,
            "Actions disabled in replay/read-only mode"
        );
        let mut output = String::new();
        for (program, args) in &self.commands {
            let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            let result = discovery::command_with_stderr(program, &refs, Duration::from_secs(20))?;
            output.push_str(&format!("{program}: {}\n{result}\n", args.join(" ")));
            #[cfg(windows)]
            if program == "sc.exe"
                && args.first().is_some_and(|a| a == "stop")
                && self.commands.len() > 1
            {
                let start = std::time::Instant::now();
                loop {
                    let state =
                        discovery::command("sc.exe", &["query", &args[1]], Duration::from_secs(2))?;
                    if state
                        .lines()
                        .any(|l| l.contains("STATE") && l.split_whitespace().nth(2) == Some("1"))
                    {
                        break;
                    }
                    ensure!(
                        start.elapsed() < Duration::from_secs(20),
                        "Service did not stop within 20s"
                    );
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
        if output.trim().is_empty() {
            output = "Completed".into();
        }
        Ok(output)
    }
}

pub fn service_actions() -> &'static [&'static str] {
    #[cfg(target_os = "linux")]
    {
        &[
            "status",
            "configuration",
            "dependencies",
            "reverse-dependencies",
            "start",
            "stop",
            "restart",
            "reload",
            "enable",
            "disable",
            "mask",
            "unmask",
            "reset-failed",
            "daemon-reload",
        ]
    }
    #[cfg(windows)]
    {
        &[
            "status",
            "configuration",
            "dependencies",
            "recovery-policy",
            "start",
            "stop",
            "restart",
            "pause",
            "continue",
            "enable",
            "disable",
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "status",
            "configuration",
            "start",
            "restart",
            "unload",
            "load",
            "enable",
            "disable",
        ]
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        &[]
    }
}
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.chars().any(char::is_control)
}
pub fn service_plan(service: &Service, action: &str) -> Result<Plan> {
    ensure!(valid_name(&service.name), "Invalid service identity");
    ensure!(
        service_actions().contains(&action),
        "Unsupported service action"
    );
    let changes_state = !matches!(
        action,
        "status" | "configuration" | "dependencies" | "reverse-dependencies" | "recovery-policy"
    );
    let mut commands: Vec<(String, Vec<String>)> = vec![];
    #[cfg(target_os = "linux")]
    {
        ensure!(
            ["user", "system"].contains(&service.scope.as_str()),
            "Unknown service scope"
        );
        let verb = match action {
            "status" => "show",
            "configuration" => "cat",
            "dependencies" | "reverse-dependencies" => "list-dependencies",
            other => other,
        };
        let mut args = vec![
            format!("--{}", service.scope),
            "--no-pager".into(),
            "--no-ask-password".into(),
            verb.into(),
        ];
        if action == "reverse-dependencies" {
            args.push("--reverse".into());
        }
        if action != "daemon-reload" {
            args.extend(["--".into(), service.name.clone()]);
        }
        commands.push(("systemctl".into(), args));
    }
    #[cfg(windows)]
    {
        let mut add = |verb: &str, tail: Vec<String>| {
            let mut args = vec![verb.into(), service.name.clone()];
            args.extend(tail);
            commands.push(("sc.exe".into(), args));
        };
        match action {
            "status" => add("queryex", vec![]),
            "configuration" => add("qc", vec![]),
            "dependencies" => add("enumdepend", vec![]),
            "recovery-policy" => add("qfailure", vec![]),
            "enable" => add("config", vec!["start=".into(), "auto".into()]),
            "disable" => add("config", vec!["start=".into(), "disabled".into()]),
            "restart" => {
                if service.active == "active" {
                    add("stop", vec![]);
                }
                add("start", vec![]);
            }
            verb => add(verb, vec![]),
        }
    }
    #[cfg(target_os = "macos")]
    {
        let domain = if service.scope == "system" {
            "system".into()
        } else {
            format!("gui/{}", unsafe { libc::geteuid() })
        };
        let target = format!("{domain}/{}", service.name);
        let args = match action {
            "status" => vec!["print".into(), target],
            "configuration" => {
                let path = service
                    .extra
                    .get("plist")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("No readable plist path"))?;
                return Ok(Plan {
                    title: format!("Configuration {}", service.name),
                    commands: vec![("plutil".into(), vec!["-p".into(), path.into()])],
                    changes_state: false,
                });
            }
            "start" => vec!["kickstart".into(), target],
            "restart" => vec!["kickstart".into(), "-k".into(), target],
            "unload" => vec!["bootout".into(), target],
            "load" => {
                let path = service
                    .extra
                    .get("plist")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("No plist path; use start for loaded services")
                    })?;
                vec!["bootstrap".into(), domain, path.into()]
            }
            verb => vec![verb.into(), target],
        };
        commands.push(("launchctl".into(), args));
    }
    Ok(Plan {
        title: if action == "daemon-reload" {
            format!(
                "Reload ALL {} unit definitions · daemon-reload",
                service.scope
            )
        } else {
            format!("Service {} / {} · {action}", service.scope, service.name)
        },
        commands,
        changes_state,
    })
}
pub fn container_actions() -> &'static [&'static str] {
    &[
        "inspect",
        "processes",
        "changes",
        "start",
        "stop",
        "restart",
        "pause",
        "unpause",
        "kill",
        "remove",
    ]
}

pub fn action_key(action: &str) -> Option<char> {
    Some(match action {
        "status" | "inspect" => 'i',
        "configuration" => 'c',
        "dependencies" => 'd',
        "reverse-dependencies" => 'D',
        "recovery-policy" => 'v',
        "start" => 's',
        "stop" => 't',
        "restart" => 'r',
        "reload" => 'l',
        "load" => 'L',
        "unload" => 'U',
        "enable" => 'e',
        "disable" => 'b',
        "mask" => 'm',
        "unmask" => 'u',
        "reset-failed" => 'f',
        "daemon-reload" => 'R',
        "pause" => 'p',
        "unpause" | "continue" => 'u',
        "kill" => 'k',
        "remove" => 'R',
        "processes" => 'T',
        "changes" => 'd',
        _ => return None,
    })
}
pub fn container_plan(container: &Container, action: &str) -> Result<Plan> {
    ensure!(
        ["docker", "podman"].contains(&container.runtime.as_str()),
        "Unknown container runtime"
    );
    ensure!(
        container.id.len() >= 12 && container.id.chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid container ID"
    );
    ensure!(
        container_actions().contains(&action),
        "Unsupported container action"
    );
    let verb = match action {
        "processes" => "top",
        "changes" => "diff",
        "remove" => "rm",
        other => other,
    };
    let args = vec![verb.into(), container.id.clone()];
    Ok(Plan {
        title: format!(
            "Container {} / {} · {action}",
            container.runtime, container.name
        ),
        commands: vec![(container.runtime.clone(), args)],
        changes_state: !matches!(action, "inspect" | "processes" | "changes"),
    })
}
