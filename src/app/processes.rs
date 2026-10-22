use super::{App, Sort, View};
use crate::model::Process;
use std::collections::{HashMap, HashSet};
#[derive(Default)]
pub struct ProgramGroup {
    pub name: String,
    pub members: Vec<(u32, u64)>,
    pub cpu: f32,
    pub memory: u64,
    pub read_per_sec: f64,
    pub write_per_sec: f64,
    pub gpu_memory: Option<u64>,
}
impl App {
    pub fn process_grouped(&self) -> bool {
        self.view == View::Processes && self.config.display.group_processes
    }
    pub fn program_groups(&self) -> Vec<ProgramGroup> {
        let mut groups = HashMap::<String, ProgramGroup>::new();
        for (index, _) in self.process_rows() {
            let process = &self.snapshot.processes[index];
            if process.thread {
                continue;
            }
            let group = groups
                .entry(process.name.clone())
                .or_insert_with(|| ProgramGroup {
                    name: process.name.clone(),
                    ..Default::default()
                });
            group.members.push((process.pid, process.start_time));
            group.cpu += process.cpu;
            group.memory = group.memory.saturating_add(process.memory);
            group.read_per_sec += process.read_per_sec;
            group.write_per_sec += process.write_per_sec;
            if let Some(memory) = process.gpu_memory {
                group.gpu_memory = Some(group.gpu_memory.unwrap_or(0).saturating_add(memory));
            }
        }
        let mut groups: Vec<_> = groups.into_values().collect();
        groups.sort_by(|a, b| {
            let order = match self.sort {
                Sort::Memory => b.memory.cmp(&a.memory),
                Sort::Name => a.name.cmp(&b.name),
                Sort::Read => b.read_per_sec.total_cmp(&a.read_per_sec),
                Sort::Write => b.write_per_sec.total_cmp(&a.write_per_sec),
                Sort::GpuMemory => b.gpu_memory.cmp(&a.gpu_memory),
                _ => b.cpu.total_cmp(&a.cpu),
            }
            .then(a.name.cmp(&b.name));
            if self.reverse { order.reverse() } else { order }
        });
        groups
    }
    pub fn open_program_group(&mut self) {
        let group = self.program_groups().into_iter().nth(self.selected);
        if let Some(group) = group {
            self.object_scope = Some(super::tag_context::ObjectScope {
                title: format!("Program {}", group.name),
                processes: group.members.into_iter().collect(),
                connections: Default::default(),
            });
            self.config.display.group_processes = false;
            self.tree = false;
            self.selected = 0;
            self.table_offset = 0;
            self.clear_process_selection();
        }
    }
    pub fn toggle_tag(&mut self) {
        if let Some(identity) = self.selected_process().map(|p| (p.pid, p.start_time))
            && !self.tagged.insert(identity)
        {
            self.tagged.remove(&identity);
        }
    }
    pub fn tag_children(&mut self) {
        if let Some(root) = self.selected_process().map(|p| p.pid) {
            let mut todo = vec![root];
            let mut seen = HashSet::new();
            while let Some(pid) = todo.pop() {
                if !seen.insert(pid) {
                    continue;
                }
                if let Some(p) = self.snapshot.processes.iter().find(|p| p.pid == pid) {
                    self.tagged.insert((p.pid, p.start_time));
                }
                todo.extend(
                    self.snapshot
                        .processes
                        .iter()
                        .filter(|p| p.parent == Some(pid))
                        .map(|p| p.pid),
                );
            }
        }
    }
    pub fn process_rows(&self) -> Vec<(usize, usize)> {
        let processes = &self.snapshot.processes;
        let query = crate::filter::Query::parse(&self.filter);
        let own_user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default();
        let selected_gpu_id = self.snapshot.gpus.get(self.selected_gpu).map(|g| g.id);
        let matches = |p: &Process| {
            (!self.gpu_only || !p.gpu_ids.is_empty())
                && self
                    .object_scope
                    .as_ref()
                    .is_none_or(|scope| scope.processes.contains(&(p.pid, p.start_time)))
                && (!self.own_only || p.user == own_user)
                && (self.config.show_threads || !p.thread)
                && (self.config.show_kernel_threads || !p.kernel_thread)
                && (self.view != View::Gpu
                    || selected_gpu_id.is_some_and(|id| p.gpu_ids.contains(&id)))
                && query.matches(p)
        };
        let compare = |a: &usize, b: &usize| {
            let a = &processes[*a];
            let b = &processes[*b];
            if self.selected_process.is_some() && !self.process_order.is_empty() {
                let rank = |p: &Process| {
                    self.process_order
                        .get(&(p.pid, p.start_time))
                        .copied()
                        .unwrap_or(usize::MAX)
                };
                let order = rank(a).cmp(&rank(b));
                if !order.is_eq() {
                    return order;
                }
            }
            let order = match self.sort {
                Sort::Cpu => b.cpu.total_cmp(&a.cpu),
                Sort::Memory => b.memory.cmp(&a.memory),
                Sort::GpuMemory => self.gpu_metrics(b).0.cmp(&self.gpu_metrics(a).0),
                Sort::Io => (b.read_per_sec + b.write_per_sec)
                    .total_cmp(&(a.read_per_sec + a.write_per_sec)),
                Sort::Pid => a.pid.cmp(&b.pid),
                Sort::Name => a.name.cmp(&b.name),
                Sort::User => a.user.cmp(&b.user),
                Sort::Gpu => self.gpu_metrics(b).1.cmp(&self.gpu_metrics(a).1),
                Sort::Read => b.read_per_sec.total_cmp(&a.read_per_sec),
                Sort::Write => b.write_per_sec.total_cmp(&a.write_per_sec),
                Sort::Vms => b.virtual_memory.cmp(&a.virtual_memory),
                Sort::Nice => a.nice.cmp(&b.nice),
                Sort::Time => b.cpu_time_ms.cmp(&a.cpu_time_ms),
                Sort::Rx => b
                    .rx_per_sec
                    .partial_cmp(&a.rx_per_sec)
                    .unwrap_or(std::cmp::Ordering::Equal),
                Sort::Tx => b
                    .tx_per_sec
                    .partial_cmp(&a.tx_per_sec)
                    .unwrap_or(std::cmp::Ordering::Equal),
                Sort::ReadTotal => b.read_total.cmp(&a.read_total),
                Sort::WriteTotal => b.write_total.cmp(&a.write_total),
                Sort::State => a.state.cmp(&b.state),
                Sort::Field(index) => {
                    let key = crate::fields::PROCESS_COLUMNS[index as usize];
                    let (a, b) = (
                        crate::fields::process_metric(a, &self.snapshot, key)
                            .filter(|v| !v.is_null()),
                        crate::fields::process_metric(b, &self.snapshot, key)
                            .filter(|v| !v.is_null()),
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let order = if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                                a.total_cmp(&b)
                            } else {
                                a.to_string().cmp(&b.to_string())
                            };
                            if self.sort.descending() {
                                order.reverse()
                            } else {
                                order
                            }
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                }
            }
            .then(a.pid.cmp(&b.pid));
            if self.reverse { order.reverse() } else { order }
        };
        let mut indices: Vec<_> = (0..processes.len())
            .filter(|i| matches(&processes[*i]))
            .collect();
        indices.sort_by(compare);
        if !self.tree || self.process_grouped() {
            return indices.into_iter().map(|i| (i, 0)).collect();
        }
        let by_pid: HashMap<_, _> = processes
            .iter()
            .enumerate()
            .map(|(i, p)| (p.pid, i))
            .collect();
        let mut included: HashSet<usize> = indices.into_iter().collect();
        // Preserve ancestor context when filtering, with a bound against corrupt cycles.
        for i in included.clone() {
            let mut next = processes[i].parent;
            let mut visited = HashSet::new();
            while let Some(pid) = next {
                if !visited.insert(pid) {
                    break;
                }
                let Some(&parent) = by_pid.get(&pid) else {
                    break;
                };
                included.insert(parent);
                next = processes[parent].parent;
            }
        }
        let mut children: HashMap<Option<u32>, Vec<usize>> = HashMap::new();
        for &i in &included {
            let parent = processes[i].parent.filter(|pid| {
                by_pid
                    .get(pid)
                    .is_some_and(|index| included.contains(index))
                    && *pid != processes[i].pid
            });
            children.entry(parent).or_default().push(i);
        }
        for group in children.values_mut() {
            group.sort_by(compare);
        }
        let mut result = vec![];
        let mut seen = HashSet::new();
        let mut stack: Vec<_> = children
            .get(&None)
            .into_iter()
            .flatten()
            .rev()
            .map(|i| (*i, 0))
            .collect();
        let mut remaining: Vec<_> = included.into_iter().collect();
        remaining.sort_by(compare);
        loop {
            while let Some((i, depth)) = stack.pop() {
                if !seen.insert(i) {
                    continue;
                }
                result.push((i, depth));
                if !self.collapsed.contains(&processes[i].pid) && depth < 128 {
                    if let Some(kids) = children.get(&Some(processes[i].pid)) {
                        stack.extend(kids.iter().rev().map(|i| (*i, depth + 1)));
                    }
                } else if let Some(kids) = children.get(&Some(processes[i].pid)) {
                    let mut hidden = kids.clone();
                    while let Some(child) = hidden.pop() {
                        if seen.insert(child)
                            && let Some(kids) = children.get(&Some(processes[child].pid))
                        {
                            hidden.extend(kids);
                        }
                    }
                }
            }
            let Some(&orphan) = remaining.iter().find(|i| !seen.contains(i)) else {
                break;
            };
            stack.push((orphan, 0));
        }
        result
    }

    pub fn gpu_metrics(&self, p: &Process) -> (Option<u64>, Option<u32>) {
        if self.view == View::Gpu {
            self.snapshot
                .gpus
                .get(self.selected_gpu)
                .and_then(|g| g.processes.iter().find(|v| v.pid == p.pid))
                .map(|v| (v.memory, v.utilization))
                .unwrap_or((None, None))
        } else {
            (p.gpu_memory, p.gpu_percent)
        }
    }
    pub fn selected_process(&self) -> Option<&Process> {
        if self.process_grouped() {
            return None;
        }
        let identity = self.selected_process?;
        self.snapshot
            .processes
            .iter()
            .find(|p| (p.pid, p.start_time) == identity)
    }
}
