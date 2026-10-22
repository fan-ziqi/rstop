use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: u64,
    pub condition: String,
    pub active: bool,
}
#[derive(Default)]
pub struct Events {
    pub history: VecDeque<Event>,
    pending: HashMap<String, u64>,
    active: HashSet<String>,
}
impl Events {
    pub fn update(
        &mut self,
        now: u64,
        conditions: &[String],
        hold: u64,
        capacity: usize,
    ) -> Vec<Event> {
        let descriptions: HashMap<String, String> = conditions
            .iter()
            .map(|condition| (condition_key(condition), condition.clone()))
            .collect();
        let current: HashSet<_> = descriptions.keys().cloned().collect();
        let mut changes = vec![];
        self.pending
            .retain(|condition, _| current.contains(condition));
        for condition in &current {
            let since = self.pending.entry(condition.clone()).or_insert(now);
            if now.saturating_sub(*since) >= hold && self.active.insert(condition.clone()) {
                changes.push(Event {
                    timestamp: now,
                    condition: descriptions
                        .get(condition)
                        .cloned()
                        .unwrap_or_else(|| condition.clone()),
                    active: true,
                });
            }
        }
        let cleared: Vec<_> = self.active.difference(&current).cloned().collect();
        for condition in cleared {
            self.active.remove(&condition);
            changes.push(Event {
                timestamp: now,
                condition,
                active: false,
            });
        }
        changes.sort_by(|a, b| a.condition.cmp(&b.condition));
        self.history.extend(changes.clone());
        while self.history.len() > capacity {
            self.history.pop_front();
        }
        changes
    }
}

fn condition_key(condition: &str) -> String {
    if condition.starts_with("CPU ") {
        return "CPU".into();
    }
    if condition.starts_with("Memory ") {
        return "Memory".into();
    }
    if condition.starts_with("GPU ") && condition.contains("hot:") {
        return condition.split(':').next().unwrap_or(condition).into();
    }
    if condition.starts_with("Container ") {
        return condition.split(": ").next().unwrap_or(condition).into();
    }
    if let Some((name, _)) = condition.split_once(" pressure ") {
        return format!("{name} pressure");
    }
    condition.into()
}
