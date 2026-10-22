use super::{App, View};
use crate::{connections::Connection, logs, tags::Tag};
use crossterm::event::KeyCode;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ObjectScope {
    pub title: String,
    pub processes: HashSet<(u32, u64)>,
    pub connections: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagAction {
    Evidence,
    Processes,
    Connections,
    Gpu,
    Logs,
    Files,
    Environment,
    Runtime,
    Storage,
    Test,
    Filter,
    Memory,
    Cpu,
    NetworkSettings,
    Service,
    Container,
    Ros,
}

#[derive(Clone, Debug)]
pub struct TagContext {
    pub title: String,
    pub tags: Vec<Tag>,
    pub scope: ObjectScope,
    pub process: Option<crate::model::Process>,
    pub gpu_id: Option<u32>,
    pub log_target: Option<logs::Target>,
    pub endpoint: Option<String>,
    pub replay: bool,
    pub service: Option<(String, String)>,
    pub container: Option<(String, String)>,
}

impl TagContext {
    pub fn actions(&self) -> Vec<(String, KeyCode, TagAction)> {
        use TagAction::*;
        let mut actions = vec![("Why these labels?".into(), KeyCode::Char('e'), Evidence)];
        use crate::tags::{Capability as C, supports};
        if !self.tags.is_empty() {
            actions.push((
                format!("Filter by label… ({})", self.tags.len()),
                KeyCode::Char('f'),
                Filter,
            ));
        }
        if !self.scope.processes.is_empty() {
            actions.push((
                format!("Related processes ({})", self.scope.processes.len()),
                KeyCode::Char('p'),
                Processes,
            ));
        }
        if !self.scope.connections.is_empty() || supports(&self.tags, C::Network) {
            actions.push((
                format!("Related connections ({})", self.scope.connections.len()),
                KeyCode::Char('n'),
                Connections,
            ));
        }
        if self.gpu_id.is_some() {
            actions.push(("View this GPU".into(), KeyCode::Char('g'), Gpu));
        }
        if !self.replay {
            if self.process.is_some() && self.tags.iter().any(|tag| tag.key == "ros") {
                actions.push((
                    "ROS topics / nodes / info / echo…".into(),
                    KeyCode::Char('b'),
                    Ros,
                ));
            }
            if self.log_target.is_some() {
                actions.push(("Output / workload logs".into(), KeyCode::Char('l'), Logs));
            }
            if self.endpoint.is_some() {
                actions.push(("Test TCP endpoint…".into(), KeyCode::Char('t'), Test));
            }
            if self.process.is_some() && cfg!(target_os = "linux") {
                if supports(&self.tags, C::Communication) {
                    actions.push((
                        "Communication environment".into(),
                        KeyCode::Char('c'),
                        Environment,
                    ));
                }
                for (cap, label, key, action) in [
                    (C::Runtime, "Loaded runtime components", 'r', Runtime),
                    (C::Files, "Open files", 'o', Files),
                    (C::Memory, "Memory / limits / OOM score", 'm', Memory),
                    (C::Cpu, "CPU scheduling / affinity", 'a', Cpu),
                ] {
                    if supports(&self.tags, cap) {
                        actions.push((label.into(), KeyCode::Char(key), action));
                    }
                }
            }
            if supports(&self.tags, C::Network) {
                actions.push((
                    "Host routes / DNS settings".into(),
                    KeyCode::Char('d'),
                    NetworkSettings,
                ));
            }
        }
        if supports(&self.tags, C::Storage) {
            actions.push(("Host storage / IO".into(), KeyCode::Char('s'), Storage));
        }
        if self.service.is_some() {
            actions.push(("Owning service".into(), KeyCode::Char('u'), Service));
        }
        if self.container.is_some() {
            actions.push(("Owning container".into(), KeyCode::Char('v'), Container));
        }

        actions
    }
}

fn tcp_target(c: &Connection) -> Option<String> {
    if !c.protocol.to_lowercase().starts_with("tcp") {
        return None;
    }
    let endpoint = if c.state == "LISTEN" {
        &c.local
    } else if c.state == "ESTABLISHED" {
        &c.remote
    } else {
        return None;
    };
    let mut address = endpoint.parse::<std::net::SocketAddr>().ok()?;
    if address.port() == 0 {
        return None;
    }
    if address.ip().is_unspecified() {
        address.set_ip(if address.is_ipv4() {
            std::net::Ipv4Addr::LOCALHOST.into()
        } else {
            std::net::Ipv6Addr::LOCALHOST.into()
        });
    }
    Some(address.to_string())
}

impl App {
    pub fn has_tag_target(&self) -> bool {
        if self.has_process_table() {
            return self.selected_process.is_some();
        }
        matches!(self.view, View::Containers | View::Services)
            || self.view == View::Network && self.network_section == 0
    }
    /// Build from stable sampled identities; a menu retains this object as rows reorder.
    pub fn tag_context(&self) -> Option<TagContext> {
        let mut members = HashSet::new();
        let mut socket_ids = HashSet::new();
        let (title, tags, primary, log_target, group) = if self.has_process_table() {
            let p = self.selected_process()?;
            let group = self
                .applications
                .iter()
                .find(|a| a.members.contains(&(p.pid, p.start_time)));
            members.insert((p.pid, p.start_time));
            (
                format!("{} · PID {}", p.name, p.pid),
                p.tags.clone(),
                Some(p.clone()),
                Some(logs::Target::process(p, &self.snapshot)),
                group,
            )
        } else {
            match self.view {
                View::Services => {
                    let s = *self.filtered_services().get(self.selected)?;
                    let group = self.applications.iter().find(|a| {
                        a.service.is_some_and(|i| {
                            self.snapshot
                                .inventory
                                .services
                                .get(i)
                                .is_some_and(|v| v.name == s.name && v.scope == s.scope)
                        })
                    });
                    (
                        s.name.clone(),
                        s.tags.clone(),
                        None,
                        logs::Target::service(s, self.config.log_lines),
                        group,
                    )
                }
                View::Containers => {
                    let c = *self.filtered_containers().get(self.selected)?;
                    let group = self.applications.iter().find(|a| {
                        a.container.is_some_and(|i| {
                            self.snapshot
                                .inventory
                                .containers
                                .get(i)
                                .is_some_and(|v| v.id == c.id && v.runtime == c.runtime)
                        })
                    });
                    (
                        c.name.clone(),
                        c.tags.clone(),
                        None,
                        Some(logs::Target::container(c, self.config.log_lines)),
                        group,
                    )
                }
                View::Network if self.network_section == 0 => {
                    let c = *self.filtered_connections().get(self.selected)?;
                    socket_ids.insert(c.id.clone());
                    for pid in &c.pids {
                        if let Some(p) = self.snapshot.processes.iter().find(|p| {
                            p.pid == *pid
                                && !p.thread
                                && (self.snapshot.connections.owners_sampled_at == 0
                                    || p.start_time <= self.snapshot.connections.owners_sampled_at)
                        }) {
                            members.insert((p.pid, p.start_time));
                        }
                    }
                    let p = if members.len() == 1 {
                        self.snapshot
                            .processes
                            .iter()
                            .find(|p| members.contains(&(p.pid, p.start_time)))
                            .cloned()
                    } else {
                        None
                    };
                    let log = p.as_ref().map(|p| logs::Target::process(p, &self.snapshot));
                    (
                        format!("{} → {}", c.local, c.remote),
                        c.tags.clone(),
                        p,
                        log,
                        None,
                    )
                }
                _ => return None,
            }
        };
        let group = group.or_else(|| {
            (members.len() == 1)
                .then(|| {
                    self.applications
                        .iter()
                        .find(|a| a.members.iter().any(|identity| members.contains(identity)))
                })
                .flatten()
        });
        let service = group
            .and_then(|g| g.service)
            .and_then(|i| self.snapshot.inventory.services.get(i))
            .map(|s| (s.scope.clone(), s.name.clone()));
        let container = group
            .and_then(|g| g.container)
            .and_then(|i| self.snapshot.inventory.containers.get(i))
            .map(|c| (c.runtime.clone(), c.id.clone()));
        if let Some(group) = group {
            members.extend(&group.members);
            socket_ids.extend(
                group
                    .sockets
                    .iter()
                    .filter_map(|i| self.snapshot.connections.connections.get(*i))
                    .map(|c| c.id.clone()),
            );
        }
        let process = primary.or_else(|| {
            if members.len() == 1 {
                self.snapshot
                    .processes
                    .iter()
                    .find(|p| members.contains(&(p.pid, p.start_time)))
                    .cloned()
            } else {
                None
            }
        });
        let gpu_id = members
            .iter()
            .filter_map(|identity| {
                self.snapshot
                    .processes
                    .iter()
                    .find(|p| (p.pid, p.start_time) == *identity)
            })
            .flat_map(|p| p.gpu_ids.iter())
            .filter(|id| self.snapshot.gpus.iter().any(|g| g.id == **id))
            .copied()
            .min();
        let endpoint = self
            .snapshot
            .connections
            .connections
            .iter()
            .filter(|c| socket_ids.contains(&c.id))
            .find_map(tcp_target);
        Some(TagContext {
            title: title.clone(),
            tags,
            scope: ObjectScope {
                title,
                processes: members,
                connections: socket_ids,
            },
            process,
            gpu_id,
            log_target,
            endpoint,
            replay: self.replay,
            service,
            container,
        })
    }
    pub fn open_tag_scope(&mut self, context: &TagContext, network: bool) {
        self.set_view(if network {
            View::Network
        } else {
            View::Processes
        });
        if network {
            self.set_network_section(0);
        } else {
            self.gpu_only = false;
            self.own_only = false;
            self.tree = false;
        }
        self.status = format!(
            "Related to {} · reopen this view for all objects",
            context.title
        );
        self.object_scope = Some(context.scope.clone());
    }
}
