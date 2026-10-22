use super::Input;
use crate::runtime::inspect::open_report;
use crossterm::event::{KeyCode, KeyEvent};
use rstop::{
    app::{
        App,
        ros::{Action, Browser, Page},
    },
    model::Process,
    ros_tools::{Query, Reply, Session},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(super) enum ResultPage {
    Ready(Box<Session>),
    Query(Reply),
}

pub(crate) struct Job {
    generation: u64,
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<anyhow::Result<ResultPage>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Input<'_> {
    fn ros_task(
        &mut self,
        app: &mut App,
        title: &str,
        work: impl FnOnce(&Arc<AtomicBool>) -> anyhow::Result<ResultPage> + Send + 'static,
    ) {
        self.ros_job = None;
        app.ui_menu = None;
        open_report(app, title, "Loading… · Esc / click outside to stop".into());
        app.detail_pending = true;
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work(&worker_cancel));
        });
        self.ros_job = Some(Job {
            generation: app.detail_generation,
            cancel,
            rx,
        });
    }

    pub(super) fn open_ros(&mut self, app: &mut App, process: Process) {
        if app.replay {
            app.status = "Live ROS tools are disabled in replay".into();
            return;
        }
        app.ros_browser = None;
        self.ros_task(app, "ROS COMMUNICATION CONTEXT", move |_| {
            Session::from_process(process)
                .map(Box::new)
                .map(ResultPage::Ready)
        });
    }

    pub(super) fn restore_ros(&mut self, app: &mut App) {
        self.ros_job = None;
        if let Some(browser) = &app.ros_browser {
            app.ui_menu = Some(browser.menu(0));
        }
    }

    /// Async replies are tied to the modal generation, so canceled queries cannot reopen a window.
    pub fn poll_ros(&mut self, app: &mut App) -> bool {
        let Some(job) = &self.ros_job else {
            return false;
        };
        if job.generation != app.detail_generation || app.replay {
            self.ros_job = None;
            return false;
        }
        let result = match job.rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(anyhow::anyhow!("ROS query worker stopped"))
            }
        };
        self.ros_job = None;
        app.detail_pending = false;
        match result {
            Ok(ResultPage::Ready(session)) => {
                app.ros_browser = Some(Browser::new(*session));
                app.detail = None;
                self.restore_ros(app);
            }
            Ok(ResultPage::Query(Reply::Entries(kind, entries))) => {
                if let Some(browser) = &mut app.ros_browser {
                    // Refresh replaces the current list instead of adding another back-stack entry.
                    if matches!(browser.pages.last(), Some(Page::List(k, _)) if k == &kind) {
                        browser.pages.pop();
                    }
                    browser.push(Page::List(kind, entries));
                    app.detail = None;
                    self.restore_ros(app);
                }
            }
            Ok(ResultPage::Query(Reply::Report(body))) => {
                if let Some((_, text)) = &mut app.detail {
                    *text = body;
                }
            }
            Err(error) => {
                if let Some((_, text)) = &mut app.detail {
                    *text = format!("Cannot complete ROS check: {error}");
                }
            }
        }
        true
    }

    pub(super) fn ros_filter_key(&mut self, app: &mut App, key: KeyEvent) -> bool {
        let Some(browser) = &mut app.ros_browser else {
            return false;
        };
        if !matches!(browser.pages.last(), Some(Page::List(_, _))) {
            return false;
        }
        if browser.searching {
            match key.code {
                KeyCode::Esc => {
                    browser.searching = false;
                    browser.filter.clear();
                }
                KeyCode::Enter => browser.searching = false,
                _ => {
                    app.editor.edit(&mut browser.filter, key);
                }
            }
        } else if key.code == KeyCode::Char('/') {
            browser.searching = true;
            app.editor.reset();
        } else {
            return false;
        }
        app.ui_menu = Some(browser.menu(0));
        true
    }

    pub(super) fn ros_back(&mut self, app: &mut App) {
        self.ros_job = None;
        if app.ros_browser.as_mut().is_some_and(Browser::back) {
            self.restore_ros(app);
        } else {
            app.ros_browser = None;
        }
    }

    pub(super) fn ros_action(&mut self, app: &mut App, index: usize) {
        if app.replay {
            app.ros_browser = None;
            return;
        }
        let Some(browser) = &app.ros_browser else {
            return;
        };
        let Some((_, _, action)) = browser.actions().get(index).cloned() else {
            return;
        };
        match action {
            Action::Open(page) => {
                if let Some(browser) = &mut app.ros_browser {
                    browser.push(page);
                }
                self.restore_ros(app);
            }
            Action::Query(query) => {
                let session = browser.session.clone();
                let title = format!(
                    "{} · {}",
                    session.scope(),
                    match &query {
                        Query::List(_) => "Discovering graph".into(),
                        Query::TopicInfo(n) | Query::NodeInfo(n) | Query::ServiceInfo(n) =>
                            format!("Info {n}"),
                        Query::Echo(n, _) => format!("Echo {n}"),
                        Query::Hz(n) => format!("Hz {n}"),
                        Query::Definition(e) => format!("Definition {}", e.name),
                        Query::Parameter(n, p) => format!("Parameter {n} {p}"),
                        Query::Context => "Communication context".into(),
                    }
                );
                self.ros_task(app, &title, move |cancel| {
                    session.query(query, cancel).map(ResultPage::Query)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{press, run};
    use super::*;
    use rstop::{
        app::MenuKind,
        ros_tools::{Entry, ListKind},
    };
    use std::time::{Duration, Instant};

    fn browser() -> Browser {
        Browser::new(Session {
            process: Process {
                pid: 42,
                start_time: 7,
                ..Default::default()
            },
            version: 2,
            environment: Default::default(),
            cwd: "/".into(),
        })
    }
    fn topic() -> Entry {
        Entry {
            name: "/robot/chatter".into(),
            types: vec!["std_msgs/msg/String".into()],
        }
    }

    #[test]
    fn graph_lists_search_and_outside_clicks_close_only_the_current_layer_at_all_sizes() {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            run(|input, app| {
                let mut b = browser();
                b.push(Page::List(
                    ListKind::Topics,
                    vec![
                        topic(),
                        Entry {
                            name: "/scan".into(),
                            types: vec![],
                        },
                    ],
                ));
                app.ros_browser = Some(b);
                input.restore_ros(app);
                press(input, app, KeyCode::Char('/'));
                for c in "chatter".chars() {
                    press(input, app, KeyCode::Char(c));
                }
                assert_eq!(app.ui_menu.as_ref().unwrap().items.len(), 2); // match + refresh
                press(input, app, KeyCode::Enter); // finish search
                press(input, app, KeyCode::Enter); // select topic
                assert!(
                    matches!(app.ros_browser.as_ref().unwrap().pages.last(), Some(Page::Topic(e)) if e.name == "/robot/chatter")
                );
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                let click = MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                };
                input.mouse(app, click);
                assert!(matches!(
                    app.ros_browser.as_ref().unwrap().pages.last(),
                    Some(Page::List(_, _))
                ));
                press(input, app, KeyCode::Char('/'));
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                input.mouse(app, click); // searching is not another popup layer
                assert!(matches!(
                    app.ros_browser.as_ref().unwrap().pages.last(),
                    Some(Page::Root)
                ));
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                input.mouse(app, click);
                assert!(app.ros_browser.is_none() && app.ui_menu.is_none());
                assert!(input.queued_key.is_none());
            });
        }
    }

    #[test]
    fn cancel_stops_query_and_late_replies_cannot_reopen_a_popup() {
        run(|input, app| {
            app.ros_browser = Some(browser());
            let canceled = Arc::new(AtomicBool::new(false));
            let worker_canceled = canceled.clone();
            input.ros_task(app, "ROS TEST", move |cancel| {
                while !cancel.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                worker_canceled.store(true, Ordering::Relaxed);
                Ok(ResultPage::Query(Reply::Report("late result".into())))
            });
            press(input, app, KeyCode::Esc);
            assert_eq!(app.ui_menu.as_ref().unwrap().kind, MenuKind::Ros);
            assert!(app.detail.is_none() && !app.detail_pending);
            let started = Instant::now();
            while !canceled.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(1) {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(canceled.load(Ordering::Relaxed));
            assert!(!input.poll_ros(app));
            press(input, app, KeyCode::Esc);
            assert!(app.ros_browser.is_none());

            input.ros_task(app, "STALE", |_| {
                Ok(ResultPage::Query(Reply::Report("stale".into())))
            });
            app.detail_generation += 1;
            app.detail = None;
            input.poll_ros(app);
            assert!(input.ros_job.is_none() && app.detail.is_none() && app.ui_menu.is_none());
        });
    }
}
