use super::Input;
use crossterm::event::KeyCode;
use rstop::app::{App, SignalTarget};
use sysinfo::Signal;

impl Input<'_> {
    pub(super) fn open_signal(&mut self, app: &mut App, key: KeyCode) {
        if app.replay || self.read_only {
            app.status = "Process actions disabled in replay/read-only mode".into();
            return;
        }
        let signal = match key {
            KeyCode::Char('k' | 'K') => Signal::Kill,
            KeyCode::Char('t') => Signal::Term,
            _ if sysinfo::SUPPORTED_SIGNALS.contains(&Signal::Term) => Signal::Term,
            _ => Signal::Kill,
        };
        if !sysinfo::SUPPORTED_SIGNALS.contains(&signal) {
            app.status =
                format!("{signal:?} is unsupported on this platform; use F9 for available signals");
            return;
        }
        let Some(process) = app.selected_process() else {
            return;
        };
        let target = SignalTarget {
            pid: process.pid,
            start_time: process.start_time,
            name: process.name.clone(),
            signal,
        };
        app.signal_targets = app
            .snapshot
            .processes
            .iter()
            .filter(|p| app.tagged.contains(&(p.pid, p.start_time)))
            .map(|p| SignalTarget {
                pid: p.pid,
                start_time: p.start_time,
                name: p.name.clone(),
                signal,
            })
            .collect();
        if app.signal_targets.is_empty() {
            app.signal_targets.push(target.clone());
        }
        app.signal_ui = Default::default();
        app.signal = Some(target);
    }
}

pub(super) fn shortcut(key: KeyCode) -> Option<Signal> {
    Some(match key {
        KeyCode::Char('i' | 'I') => Signal::Interrupt,
        KeyCode::Char('t' | 'T') => Signal::Term,
        KeyCode::Char('k' | 'K') => Signal::Kill,
        KeyCode::Char('s' | 'S') => Signal::Stop,
        KeyCode::Char('c' | 'C') => Signal::Continue,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{press, run};
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    use rstop::{
        app::View,
        model::{Gpu, Process},
    };

    #[test]
    fn process_shortcuts_preview_the_requested_signal_and_keep_navigation_separate() {
        for view in [View::Processes, View::Gpu] {
            for (key, signal) in [
                ('k', Signal::Kill),
                ('K', Signal::Kill),
                ('t', Signal::Term),
            ] {
                run(|input, app| {
                    app.snapshot.gpus.push(Gpu::default());
                    app.snapshot.processes[0].gpu_ids.push(0);
                    app.set_view(view);
                    press(input, app, KeyCode::Char(key));
                    assert!(app.signal.is_none());
                    assert!(app.status.contains("Select a process"));
                    press(input, app, KeyCode::Down);
                    let identity = app.selected_process;
                    press(input, app, KeyCode::Char(key));
                    if sysinfo::SUPPORTED_SIGNALS.contains(&signal) {
                        assert_eq!(app.signal.as_ref().unwrap().signal, signal);
                        assert_eq!(app.signal_targets.len(), 1);
                        assert_eq!(app.selected_process, identity);
                        // Repeating an action or an unrelated key must retain the preview.
                        press(input, app, KeyCode::Char(key));
                        press(input, app, KeyCode::Char('z'));
                        assert_eq!(app.signal.as_ref().unwrap().signal, signal);
                        press(input, app, KeyCode::Char('n'));
                        assert!(app.signal.is_none() && app.signal_targets.is_empty());
                    } else {
                        assert!(app.signal.is_none());
                        assert!(app.status.contains("unsupported"));
                    }
                    input.key(app, KeyEvent::new(KeyCode::Char('k'), KeyModifiers::ALT));
                    assert!(app.signal.is_none());
                    assert_eq!(app.selected_process, identity);
                    input.key(
                        app,
                        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
                    );
                    assert!(app.signal.is_none());
                });
            }
        }
    }

    #[test]
    fn signal_preview_freezes_marked_identities_and_supports_common_signal_choices() {
        run(|input, app| {
            app.snapshot.processes.push(Process {
                pid: 43,
                start_time: 100,
                name: "second-worker".into(),
                ..Default::default()
            });
            press(input, app, KeyCode::Down);
            press(input, app, KeyCode::Char(' '));
            assert!(app.tagged.contains(&(42, 0)));
            press(input, app, KeyCode::Char(' '));
            assert!(app.tagged.is_empty());
            press(input, app, KeyCode::Char('v'));
            app.tagged.extend([(43, 100), (43, 99)]);
            press(input, app, KeyCode::Char('k'));
            assert_eq!(app.signal_targets.len(), 2);
            let targets = app
                .signal_targets
                .iter()
                .map(|t| (t.pid, t.start_time))
                .collect::<Vec<_>>();
            assert_eq!(targets, [(42, 0), (43, 100)]);
            // Refreshing or changing marks cannot retarget an open confirmation.
            app.snapshot.processes[1].start_time = 101;
            app.tagged.clear();
            for (key, signal) in [
                ('i', Signal::Interrupt),
                ('t', Signal::Term),
                ('s', Signal::Stop),
                ('c', Signal::Continue),
                ('K', Signal::Kill),
            ] {
                let previous = app.signal.as_ref().unwrap().signal;
                press(input, app, KeyCode::Char(key));
                assert_eq!(
                    app.signal.as_ref().unwrap().signal,
                    if sysinfo::SUPPORTED_SIGNALS.contains(&signal) {
                        signal
                    } else {
                        previous
                    }
                );
                assert_eq!(
                    app.signal_targets
                        .iter()
                        .map(|t| (t.pid, t.start_time))
                        .collect::<Vec<_>>(),
                    targets
                );
            }
            press(input, app, KeyCode::Esc);
            assert!(app.signal_targets.is_empty());
        });
    }

    #[test]
    fn lowercase_actions_work_in_process_reports_without_stealing_editor_input() {
        for key in [
            KeyCode::Char('k'),
            KeyCode::Char('t'),
            KeyCode::Char('s'),
            KeyCode::F(9),
        ] {
            run(|input, app| {
                press(input, app, KeyCode::Down);
                app.detail = Some((
                    "Process details · PID 42".into(),
                    "sample process details".into(),
                ));
                app.detail_scroll = 10;
                press(input, app, key);
                if key == KeyCode::Char('t') && !sysinfo::SUPPORTED_SIGNALS.contains(&Signal::Term)
                {
                    assert!(app.signal.is_none());
                    assert!(app.status.contains("unsupported"), "{}", app.status);
                } else {
                    assert!(
                        app.signal.is_some(),
                        "{key:?} was swallowed by process details"
                    );
                }
                assert!(app.detail.is_some());
                press(input, app, KeyCode::Char('n'));
                assert!(app.detail.is_some() && app.signal.is_none());
                assert_eq!(app.detail_scroll, 10);
                app.detail_search = true;
                press(input, app, KeyCode::Char('k'));
                assert!(app.signal.is_none());
                assert_eq!(app.detail_filter, "k");
                press(input, app, KeyCode::Esc);
                app.detail_scroll = 10;
                input.key(app, KeyEvent::new(KeyCode::Char('k'), KeyModifiers::ALT));
                assert_eq!(app.detail_scroll, 9);
                assert!(app.signal.is_none());
            });
        }
    }

    #[test]
    fn signal_buttons_support_mouse_and_keyboard_cancellation_and_lowercase_aliases() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        run(|input, app| {
            press(input, app, KeyCode::Down);
            press(input, app, KeyCode::Char('s'));
            for (key, signal) in [
                ('i', Signal::Interrupt),
                ('I', Signal::Interrupt),
                ('t', Signal::Term),
                ('T', Signal::Term),
                ('k', Signal::Kill),
                ('K', Signal::Kill),
                ('s', Signal::Stop),
                ('S', Signal::Stop),
                ('c', Signal::Continue),
                ('C', Signal::Continue),
            ] {
                let previous = app.signal.as_ref().unwrap().signal;
                press(input, app, KeyCode::Char(key));
                assert_eq!(
                    app.signal.as_ref().unwrap().signal,
                    if sysinfo::SUPPORTED_SIGNALS.contains(&signal) {
                        signal
                    } else {
                        previous
                    }
                );
            }
            press(input, app, KeyCode::Tab);
            press(input, app, KeyCode::Tab);
            assert!(app.signal.is_some());
            press(input, app, KeyCode::Char('n'));
            assert!(app.signal.is_none());
            press(input, app, KeyCode::Char('k'));
            let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
            terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
            let rect = app
                .popup_actions
                .iter()
                .find(|(_, key)| *key == KeyCode::Char('n'))
                .unwrap()
                .0;
            input.mouse(
                app,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: rect.x + 1,
                    row: rect.y + rect.height / 2,
                    modifiers: KeyModifiers::NONE,
                },
            );
            let key = input.queued_key.take().unwrap();
            input.key(app, key);
            assert!(app.signal.is_none() && app.signal_targets.is_empty());
        });
    }

    #[test]
    fn every_supported_signal_is_reachable_and_the_highlight_tracks_arrow_navigation() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            run(|input, app| {
                press(input, app, KeyCode::Down);
                press(input, app, KeyCode::Char('s'));
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                let supported = sysinfo::SUPPORTED_SIGNALS;
                press(input, app, KeyCode::Home);
                for &expected in supported {
                    terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                    assert_eq!(app.signal.as_ref().unwrap().signal, expected);
                    let rect = app
                        .signal_ui
                        .hits
                        .iter()
                        .find(|(_, signal)| *signal == expected)
                        .expect("selected signal scrolled out of view")
                        .0;
                    assert_eq!(terminal.backend().buffer()[(rect.x, rect.y)].symbol(), "▸");
                    let label = (rect.x..rect.right())
                        .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                        .collect::<String>();
                    assert!(label.contains(rstop::app::signals::describe(expected).name));
                    press(input, app, KeyCode::Right);
                }
                assert_eq!(app.signal.as_ref().unwrap().signal, supported[0]);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                let columns = app.signal_ui.columns;
                press(input, app, KeyCode::Down);
                assert_eq!(
                    app.signal.as_ref().unwrap().signal,
                    supported[columns.min(supported.len() - 1)]
                );
                press(input, app, KeyCode::Up);
                assert_eq!(app.signal.as_ref().unwrap().signal, supported[0]);
                press(input, app, KeyCode::End);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert_eq!(
                    app.signal.as_ref().unwrap().signal,
                    *supported.last().unwrap()
                );
                if supported.len().div_ceil(columns) > app.signal_ui.rows {
                    assert!(app.signal_ui.offset > 0);
                }
                press(input, app, KeyCode::PageUp);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(
                    app.signal_ui
                        .hits
                        .iter()
                        .any(|(_, signal)| *signal == app.signal.as_ref().unwrap().signal)
                );
                press(input, app, KeyCode::PageDown);
                terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                assert!(
                    app.signal_ui
                        .hits
                        .iter()
                        .any(|(_, signal)| *signal == app.signal.as_ref().unwrap().signal)
                );
                let (rect, clicked) = app.signal_ui.hits[0];
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: rect.x + 2,
                        row: rect.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                assert_eq!(app.signal.as_ref().unwrap().signal, clicked);
                assert!(input.queued_key.is_none()); // Clicking a signal only selects it.
                assert_eq!(app.signal_targets.len(), 1);
                input.mouse(
                    app,
                    MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column: rect.x + 2,
                        row: rect.y,
                        modifiers: KeyModifiers::NONE,
                    },
                );
                let key = input
                    .queued_key
                    .take()
                    .expect("wheel must navigate the signal grid");
                input.key(app, key);
                if supported.len() > columns {
                    assert_ne!(app.signal.as_ref().unwrap().signal, clicked);
                }
            });
        }
    }

    #[test]
    fn confirmation_dialogs_use_y_or_enter_and_n_or_escape_without_text_entry() {
        for key in [
            KeyCode::Char('y'),
            KeyCode::Enter,
            KeyCode::Char('n'),
            KeyCode::Esc,
        ] {
            run(|input, app| {
                app.pending_plan = Some(rstop::management::Plan {
                    title: "No-op confirmation test".into(),
                    commands: vec![],
                    changes_state: true,
                });
                app.prompt = Some(("workload action".into(), String::new(), None));
                press(input, app, key);
                assert!(app.pending_plan.is_none() && app.prompt.is_none());
                assert_eq!(
                    app.detail_pending,
                    matches!(key, KeyCode::Char('y') | KeyCode::Enter)
                );
            });
        }
    }

    #[test]
    fn process_mutation_shortcuts_are_blocked_in_read_only_and_replay() {
        for read_only in [false, true] {
            run(|input, app| {
                press(input, app, KeyCode::Down);
                for key in ['k', 'K', 't', 's', 'n', 'N', 'a'] {
                    input.read_only = read_only;
                    app.replay = !read_only;
                    press(input, app, KeyCode::Char(key));
                    assert!(app.signal.is_none() && app.prompt.is_none());
                    assert!(app.status.contains("disabled"));
                }
                press(input, app, KeyCode::F(9));
                assert!(app.signal.is_none());
                app.replay = false;
                input.read_only = false;
                press(input, app, KeyCode::Char('k'));
                input.read_only = read_only;
                app.replay = !read_only;
                press(input, app, KeyCode::Enter);
                assert!(app.signal.is_none() && app.signal_targets.is_empty());
                assert!(app.status.contains("disabled"));
            });
        }
    }

    #[test]
    fn process_shortcuts_remain_text_in_filters_and_do_not_target_gpu_device_rows() {
        run(|input, app| {
            press(input, app, KeyCode::Char('/'));
            for key in "kKTN".chars() {
                press(input, app, KeyCode::Char(key));
            }
            assert_eq!(app.filter, "kKTN");
            assert!(app.signal.is_none() && app.prompt.is_none());
            press(input, app, KeyCode::Esc);
            press(input, app, KeyCode::Down);
            press(input, app, KeyCode::Char('N'));
            assert_eq!(app.prompt.as_ref().unwrap().0, "nice");
            press(input, app, KeyCode::Esc);
            app.snapshot.gpus.push(Gpu::default());
            app.set_view(View::Gpu);
            app.focus_panel(2);
            for key in ['k', 'K', 't', 's'] {
                press(input, app, KeyCode::Char(key));
                assert!(app.signal.is_none());
                assert_eq!(app.gpu_section, 2);
            }
            app.set_view(View::Processes);
            app.config.display.group_processes = true;
            press(input, app, KeyCode::Char('k'));
            assert!(app.signal.is_none());
        });
    }

    #[test]
    fn selected_process_actions_and_signal_targets_are_visible_in_small_terminals() {
        for (width, height) in [(60, 18), (100, 30), (160, 48)] {
            for view in [View::Processes, View::Gpu] {
                run(|input, app| {
                    app.snapshot.gpus.push(Gpu::default());
                    app.snapshot.processes[0].gpu_ids.push(0);
                    app.set_view(view);
                    press(input, app, KeyCode::Down);
                    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                    terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                    for (key, label) in [
                        (KeyCode::Char('k'), "Kill"),
                        (KeyCode::Char('t'), "Terminate"),
                        (KeyCode::Char('s'), "Signal"),
                    ] {
                        let rect = app
                            .footer_actions
                            .iter()
                            .find(|(_, k)| k.code == key)
                            .unwrap()
                            .0;
                        let text = (rect.x..rect.right())
                            .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                            .collect::<String>();
                        assert!(text.contains(label), "{width}x{height}: {text}");
                        assert!(rect.right() <= width && rect.bottom() <= height);
                    }
                    press(input, app, KeyCode::Char('k'));
                    terminal.draw(|f| rstop::ui::draw(f, app)).unwrap();
                    let rect = app.popup_rect;
                    let text = (rect.y..rect.bottom())
                        .map(|y| {
                            (rect.x..rect.right())
                                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                                .collect::<String>()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    for expected in ["Kill", "42", "worker", "Confirm", "Cancel"] {
                        assert!(
                            text.contains(expected),
                            "{width}x{height} {view:?}: missing {expected}: {text}"
                        );
                    }
                    assert!(
                        text.lines()
                            .any(|line| line.contains("42") && line.contains("worker"))
                    );
                    press(input, app, KeyCode::Esc);
                    press(input, app, KeyCode::F(6));
                    press(input, app, KeyCode::Char('k'));
                    assert_eq!(app.signal.as_ref().unwrap().signal, Signal::Kill);
                });
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn confirmed_shortcuts_signal_only_the_owned_child_and_cancel_keeps_it_alive() {
        use std::{
            process::{Child, Command},
            sync::mpsc,
            time::Duration,
        };
        struct OwnedChild(Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        for (key, confirm) in [
            ('k', KeyCode::Enter),
            ('K', KeyCode::Char('y')),
            ('t', KeyCode::Enter),
            ('i', KeyCode::Char('y')),
        ] {
            let mut child = OwnedChild(Command::new("sleep").arg("30").spawn().unwrap());
            let pid = child.0.id();
            let mut system = sysinfo::System::new();
            system.refresh_processes(
                sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
                true,
            );
            let start_time = system
                .process(sysinfo::Pid::from_u32(pid))
                .unwrap()
                .start_time();
            run(|input, app| {
                app.snapshot.processes = vec![Process {
                    pid,
                    start_time,
                    name: "owned-test-child".into(),
                    ..Default::default()
                }];
                press(input, app, KeyCode::Down);
                let (sender, receiver) = mpsc::channel();
                input.action_tx = sender;
                press(input, app, KeyCode::Char('k'));
                press(input, app, KeyCode::Esc);
                assert!(child.0.try_wait().unwrap().is_none());
                for signal_key in ['s', 'c', key] {
                    if signal_key == 'k' || signal_key == 'K' || signal_key == 't' {
                        press(input, app, KeyCode::Char(signal_key));
                    } else {
                        press(input, app, KeyCode::F(9));
                        press(input, app, KeyCode::Char(signal_key));
                    }
                    press(input, app, confirm);
                    let result = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert!(result.contains("1 succeeded, 0 failed"), "{result}");
                    if matches!(signal_key, 's' | 'c') {
                        let mut observed = false;
                        for _ in 0..100 {
                            let status =
                                std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
                            let stopped = status.lines().any(|line| line.starts_with("State:\tT"));
                            if stopped == (signal_key == 's') {
                                observed = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        assert!(
                            observed,
                            "Child did not reach expected state after {signal_key}"
                        );
                    }
                }
                for _ in 0..100 {
                    if let Some(status) = child.0.try_wait().unwrap() {
                        use std::os::unix::process::ExitStatusExt;
                        assert_eq!(
                            status.signal(),
                            Some(match key {
                                't' => libc::SIGTERM,
                                'i' => libc::SIGINT,
                                _ => libc::SIGKILL,
                            })
                        );
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                panic!("Owned child did not exit after confirmed shortcut {key}");
            });
        }
    }
}
