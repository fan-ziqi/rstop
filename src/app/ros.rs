use super::{MenuKind, UiMenu};
use crate::ros_tools::{Entry, ListKind, Query, Session};
use crossterm::event::KeyCode;

#[derive(Clone, Debug)]
pub enum Page {
    Root,
    List(ListKind, Vec<Entry>),
    Topic(Entry),
    Node(Entry),
}

#[derive(Clone, Debug)]
pub enum Action {
    Query(Query),
    Open(Page),
}

#[derive(Clone, Debug)]
pub struct Browser {
    pub session: Session,
    pub pages: Vec<Page>,
    pub filter: String,
    pub searching: bool,
}
impl Browser {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            pages: vec![Page::Root],
            filter: String::new(),
            searching: false,
        }
    }
    pub fn push(&mut self, page: Page) {
        self.pages.push(page);
        self.filter.clear();
        self.searching = false;
    }
    pub fn back(&mut self) -> bool {
        self.pages.pop();
        self.filter.clear();
        self.searching = false;
        !self.pages.is_empty()
    }
    pub fn actions(&self) -> Vec<(String, KeyCode, Action)> {
        use crate::ros_tools::Query as Q;
        use Action::*;
        let item = |name: &str, key, q| (name.into(), KeyCode::Char(key), Query(q));
        match self.pages.last().unwrap_or(&Page::Root) {
            Page::Root => {
                let mut items = vec![
                    item("Topics…", 't', Q::List(ListKind::Topics)),
                    item("Nodes…", 'n', Q::List(ListKind::Nodes)),
                    item("Services…", 's', Q::List(ListKind::Services)),
                ];
                if self.session.version == 1 {
                    items.push(item(
                        "Parameters (master)…",
                        'p',
                        Q::List(ListKind::Parameters(String::new())),
                    ));
                }
                items.push(item("Communication context", 'c', Q::Context));
                items
            }
            Page::List(kind, entries) => {
                let mut items = entries
                    .iter()
                    .filter(|e| {
                        e.display()
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                    })
                    .map(|e| {
                        (
                            e.display(),
                            KeyCode::Null,
                            match kind {
                                ListKind::Topics => Open(Page::Topic(e.clone())),
                                ListKind::Nodes => Open(Page::Node(e.clone())),
                                ListKind::Services => Query(Q::ServiceInfo(e.name.clone())),
                                ListKind::Parameters(node) => {
                                    Query(Q::Parameter(node.clone(), e.name.clone()))
                                }
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                items.push(item(
                    if entries.is_empty() {
                        "No visible entries · Refresh"
                    } else {
                        "Refresh list"
                    },
                    'r',
                    Q::List(kind.clone()),
                ));
                items
            }
            Page::Topic(e) => vec![
                item(
                    if self.session.version == 2 {
                        "Info / publishers / subscribers / QoS"
                    } else {
                        "Info / publishers / subscribers"
                    },
                    'i',
                    Q::TopicInfo(e.name.clone()),
                ),
                item(
                    "Echo first message (8 s limit)",
                    'e',
                    Q::Echo(e.name.clone(), true),
                ),
                item("Echo sample (5 s)", 's', Q::Echo(e.name.clone(), false)),
                item("Hz / receive rate (8 s)", 'h', Q::Hz(e.name.clone())),
                item("Message definition", 'd', Q::Definition(e.clone())),
            ],
            Page::Node(e) => vec![
                item(
                    "Info / publishers / subscribers / services",
                    'i',
                    Q::NodeInfo(e.name.clone()),
                ),
                item(
                    if self.session.version == 2 {
                        "Parameters…"
                    } else {
                        "Parameters (master)…"
                    },
                    'p',
                    Q::List(ListKind::Parameters(e.name.clone())),
                ),
            ],
        }
    }
    pub fn menu(&self, selected: usize) -> UiMenu {
        let title = match self.pages.last().unwrap_or(&Page::Root) {
            Page::Root => self.session.scope(),
            Page::List(kind, _) => format!(
                "{} · {} · / {}{}",
                self.session.scope(),
                match kind {
                    ListKind::Topics => "Topics",
                    ListKind::Nodes => "Nodes",
                    ListKind::Services => "Services",
                    ListKind::Parameters(_) if self.session.version == 1 => "Parameters (master)",
                    ListKind::Parameters(_) => "Parameters",
                },
                if self.filter.is_empty() && !self.searching {
                    "search"
                } else {
                    &self.filter
                },
                if self.searching { "_" } else { "" }
            ),
            Page::Topic(e) | Page::Node(e) => format!("{} · {}", self.session.scope(), e.name),
        };
        let items = self
            .actions()
            .into_iter()
            .map(|(s, k, _)| (s, k))
            .collect::<Vec<_>>();
        let selected = selected.min(items.len().saturating_sub(1));
        UiMenu::new(&title, items, selected).with_kind(MenuKind::Ros)
    }
}
