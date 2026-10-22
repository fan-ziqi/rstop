//! Discoverable actions. Menus and visible controls use the actual dispatched key.
use super::{App, View};
use crossterm::event::KeyCode;

pub fn key_name(key: KeyCode) -> String {
    match key {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Delete => "Del".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Null => String::new(),
        _ => format!("{key:?}"),
    }
}

pub fn time_label(seconds: u64) -> String {
    match seconds {
        0 => "All".into(),
        s if s < 60 => format!("{s}s"),
        s if s % 3600 == 0 => format!("{}h", s / 3600),
        s if s % 60 == 0 => format!("{}m", s / 60),
        s => format!("{}m{}s", s / 60, s % 60),
    }
}

impl App {
    pub fn process_commands(&self) -> Vec<(String, KeyCode)> {
        [
            ("Labels / related actions", KeyCode::Char('Y')),
            ("Diagnose CPU / memory / processes", KeyCode::Char('R')),
            ("Kernel / OOM messages", KeyCode::Char('J')),
            ("Sort by CPU", KeyCode::Char('P')),
            ("Sort by RAM", KeyCode::Char('M')),
            ("Inspect process", KeyCode::Enter),
            ("Full process details", KeyCode::Char('d')),
            ("Output / logs", KeyCode::F(7)),
            ("Choose process columns", KeyCode::Char('C')),
            ("Environment", KeyCode::Char('e')),
            ("Open files", KeyCode::Char('o')),
            ("Process tree", KeyCode::Char('t')),
            ("Only my processes", KeyCode::Char('u')),
            ("Only GPU processes", KeyCode::Char('g')),
            ("Show / hide threads", KeyCode::Char('H')),
            ("Show / hide kernel threads", KeyCode::Char('h')),
            ("Mark process", KeyCode::Char('v')),
            ("Mark descendant group", KeyCode::Char('V')),
            ("Clear marks", KeyCode::Char('U')),
            ("Set priority (nice)", KeyCode::Char('n')),
            ("Set I/O priority (Linux)", KeyCode::Char('I')),
            ("Set autogroup priority (Linux)", KeyCode::Char('A')),
            ("Set CPU affinity", KeyCode::Char('a')),
            ("File locks", KeyCode::Char('L')),
            ("Trace system calls", KeyCode::Char('x')),
            ("Backtrace", KeyCode::Char('B')),
            ("Send signal", KeyCode::F(9)),
        ]
        .into_iter()
        .filter(|(_, key)| {
            self.view == View::Processes || !matches!(*key, KeyCode::Char('I' | 'A'))
        })
        .map(|(label, key)| (label.into(), key))
        .collect()
    }
    pub fn network_commands(&self) -> Vec<(String, KeyCode)> {
        let mut items = vec![
            (
                if self.network_section == 0 {
                    "Connection details"
                } else {
                    "Interface details"
                }
                .into(),
                KeyCode::Enter,
            ),
            ("Test website or service".into(), KeyCode::Char('N')),
            ("Capture packets".into(), KeyCode::Char('C')),
            ("Open a packet capture".into(), KeyCode::Char('F')),
            ("Routes and DNS".into(), KeyCode::Char('O')),
            ("Diagnose network".into(), KeyCode::Char('R')),
        ];
        if self.network_section == 0 {
            items.push(("Full connection details".into(), KeyCode::Char('d')));
            items.push(("Open owning process".into(), KeyCode::Char('p')));
        }
        if self.has_tag_target() {
            items.push(("Labels and related actions".into(), KeyCode::Char('Y')));
        }
        items
    }
    pub fn tools_commands(&self) -> Vec<(String, KeyCode)> {
        if self.view == View::Network {
            return self.network_commands();
        }
        if self.has_process_table() {
            return [
                ("Inspect process", KeyCode::Enter),
                ("Full process details", KeyCode::Char('d')),
                ("Output and logs", KeyCode::F(7)),
                ("Environment", KeyCode::Char('e')),
                ("Open files", KeyCode::Char('o')),
                ("Diagnose CPU and memory", KeyCode::Char('R')),
                ("Choose process columns", KeyCode::Char('C')),
                ("Labels and related actions", KeyCode::Char('Y')),
                ("Advanced process tools", KeyCode::Char('m')),
            ]
            .into_iter()
            .map(|(label, key)| (label.into(), key))
            .collect();
        }
        let mut items = vec![
            ("Inspect selection".into(), KeyCode::Enter),
            ("Diagnose this resource".into(), KeyCode::Char('R')),
        ];
        match self.view {
            View::Services | View::Containers => items.extend([
                ("Full workload details".into(), KeyCode::Char('d')),
                ("Logs".into(), KeyCode::F(7)),
                ("Additional details".into(), KeyCode::Char('O')),
                ("Manage workload".into(), KeyCode::Char('m')),
            ]),
            View::Storage => items.extend([
                ("Find large files and folders".into(), KeyCode::Char('U')),
                ("Inodes and deleted-open files".into(), KeyCode::Char('O')),
                ("Kernel and OOM messages".into(), KeyCode::Char('J')),
            ]),
            _ => items.extend([
                ("Additional details".into(), KeyCode::Char('O')),
                ("Kernel and OOM messages".into(), KeyCode::Char('J')),
            ]),
        }
        if self.has_tag_target() {
            items.push(("Labels and related actions".into(), KeyCode::Char('Y')));
        }
        items
    }
    pub fn display_menu(&self) -> super::UiMenu {
        let (mut items, selected) = if self.view == View::Overview && !self.has_process_table() {
            let panels = ["all", "cpu", "memory", "gpu", "storage", "network"];
            (
                panels
                    .iter()
                    .map(|name| {
                        (
                            if *name == "all" {
                                "All overview panels".into()
                            } else {
                                format!("Expand {name}")
                            },
                            KeyCode::Null,
                        )
                    })
                    .collect(),
                panels
                    .iter()
                    .position(|name| *name == self.config.display.overview_panel)
                    .unwrap_or(0),
            )
        } else if self.view == View::Network {
            (
                vec![
                    ("Focus connections".into(), KeyCode::Null),
                    ("Focus interfaces".into(), KeyCode::Null),
                ],
                self.network_section,
            )
        } else if self.view == View::Gpu {
            (
                vec![
                    ("Focus GPU processes".into(), KeyCode::Null),
                    ("Focus device information".into(), KeyCode::Null),
                ],
                usize::from(self.gpu_section == 2),
            )
        } else if self.has_process_table() {
            (
                vec![
                    ("Process list".into(), KeyCode::Null),
                    ("Process tree".into(), KeyCode::Null),
                    ("Group by program".into(), KeyCode::Null),
                ]
                .into_iter()
                .filter(|(name, _)| self.view == View::Processes || name != "Group by program")
                .collect(),
                if self.process_grouped() {
                    2
                } else {
                    usize::from(self.tree)
                },
            )
        } else {
            (vec![], 0)
        };
        items.extend([
            ("Chart time window".into(), KeyCode::Char('W')),
            ("Zoom in".into(), KeyCode::Char('z')),
            ("Zoom out".into(), KeyCode::Char('Z')),
            ("Faster sampling".into(), KeyCode::Char('+')),
            ("Slower sampling".into(), KeyCode::Char('-')),
        ]);
        if self.view == View::Network {
            items.push((
                format!(
                    "{} loopback interfaces",
                    if self.config.show_loopback {
                        "Hide"
                    } else {
                        "Show"
                    }
                ),
                KeyCode::Char('i'),
            ));
        }
        if self.has_process_table() {
            items.extend([
                ("Show or hide threads".into(), KeyCode::Char('H')),
                ("Show or hide kernel threads".into(), KeyCode::Char('h')),
            ]);
        }
        if self.view == View::Gpu || self.view == View::Overview && self.snapshot.gpus.len() > 1 {
            items.extend([
                ("Previous GPU".into(), KeyCode::Char('[')),
                ("Next GPU".into(), KeyCode::Char(']')),
            ]);
        }
        super::UiMenu::new("View", items, selected).with_kind(super::MenuKind::Display)
    }
    pub fn commands(&self) -> Vec<(String, KeyCode)> {
        let mut items = vec![
            (
                "Diagnose this resource (trends / alerts / next steps)",
                KeyCode::Char('R'),
            ),
            ("Additional resource details", KeyCode::Char('O')),
            ("Kernel / OOM messages", KeyCode::Char('J')),
            ("Main menu", KeyCode::Char('?')),
            ("Help", KeyCode::F(1)),
            ("Settings", KeyCode::F(2)),
            ("View", KeyCode::F(5)),
            ("Tools", KeyCode::F(6)),
            ("Time window", KeyCode::Char('W')),
            ("Zoom in", KeyCode::Char('z')),
            ("Zoom out", KeyCode::Char('Z')),
            ("Faster sampling", KeyCode::Char('+')),
            ("Slower sampling", KeyCode::Char('-')),
            ("Save preferences", KeyCode::Char('w')),
            ("Cycle theme", KeyCode::Char('c')),
            ("Toggle loopback", KeyCode::Char('i')),
            ("Quit", KeyCode::F(10)),
        ]
        .into_iter()
        .map(|(label, key)| (label.into(), key))
        .collect::<Vec<_>>();
        if self.can_filter() {
            items.push(("Filter".into(), KeyCode::F(3)));
        }
        if self.has_tag_target() && !self.has_process_table() {
            items.push(("Labels / related actions".into(), KeyCode::Char('Y')));
        }
        if self.has_process_table()
            || matches!(self.view, View::Network | View::Containers | View::Services)
        {
            items.extend([
                ("Sort".into(), KeyCode::F(4)),
                ("Reverse sort".into(), KeyCode::Char('r')),
            ]);
        }
        if self.has_process_table() {
            items.extend(self.process_commands());
        }
        if matches!(self.view, View::Containers | View::Services) {
            items.push(("Sort by CPU".into(), KeyCode::Char('P')));
            items.push(("Sort by RAM".into(), KeyCode::Char('M')));
            items.push(("Logs".into(), KeyCode::F(7)));
            items.push(("Inspect".into(), KeyCode::Enter));
        } else if !self.has_process_table() {
            items.push(("Inspect selection".into(), KeyCode::Enter));
        }
        if self.view == View::Network {
            items.extend(self.network_commands());
        }
        if self.view == View::Storage {
            items.push(("Find large files / folders".into(), KeyCode::Char('U')));
        }
        if self.view == View::Gpu {
            items.extend([
                ("Focus GPU processes".into(), KeyCode::Char('S')),
                ("Focus device information".into(), KeyCode::Char('I')),
                ("Previous GPU".into(), KeyCode::Char('[')),
                ("Next GPU".into(), KeyCode::Char(']')),
            ]);
        }
        // The command list must include actions exposed by contextual menus too.
        // This keeps workload management and view options discoverable here.
        for (label, key) in self
            .tools_commands()
            .into_iter()
            .chain(self.display_menu().items)
        {
            if key != KeyCode::Null && !items.iter().any(|(_, existing)| *existing == key) {
                items.push((label, key));
            }
        }
        if self.replay {
            items.extend([
                ("Replay backward ten samples".into(), KeyCode::Char(',')),
                ("Replay forward ten samples".into(), KeyCode::Char('.')),
            ]);
        }
        for view in self.views() {
            items.push((format!("Open {}", view.name()), view.key()));
        }
        items
    }
}
