use super::Input;
use crate::runtime::inspect::{load_logs, open_report, task};
use rstop::app::{
    App, MenuKind, UiMenu, View,
    tag_context::{TagAction, TagContext},
};

impl Input<'_> {
    pub(super) fn open_label_actions(&mut self, app: &mut App) {
        let focused = app.clicked_label.take();
        let Some(mut context) = app.tag_context() else {
            app.status = "Select a process, connection, service or container for labels".into();
            return;
        };
        if let Some(key) = &focused {
            context.tags.retain(|tag| &tag.key == key);
            if context.tags.is_empty() {
                app.status = "Label changed; select the object again".into();
                return;
            }
        }
        let title = if focused.is_some() {
            format!("{} · {}", context.tags[0].display(), context.title)
        } else {
            format!("Labels ({}) · {}", context.tags.len(), context.title)
        };
        let items = context
            .actions()
            .into_iter()
            .map(|(label, key, _)| (label, key))
            .collect();
        app.ui_menu = Some(UiMenu::new(&title, items, 0).with_kind(MenuKind::TagActions));
        app.tag_menu_context = Some(context);
    }
    pub(super) fn label_action(&mut self, app: &mut App, context: TagContext, action: TagAction) {
        use TagAction::*;
        if !context.actions().iter().any(|(_, _, a)| *a == action) {
            return;
        }
        if matches!(
            action,
            Logs | Files | Environment | Runtime | Test | Memory | Cpu | NetworkSettings | Ros
        ) && (app.replay || context.replay)
        {
            app.status = "Live label actions are disabled in replay".into();
            return;
        }
        match action {
            Ros => {
                if let Some(process) = context.process {
                    self.open_ros(app, process);
                }
            }
            Evidence => open_report(
                app,
                &format!("LABEL EVIDENCE · {}", context.title),
                rstop::tags::explain(&context.tags),
            ),
            Processes | Connections => app.open_tag_scope(&context, action == Connections),
            Gpu => {
                let Some(index) = context
                    .gpu_id
                    .and_then(|id| app.snapshot.gpus.iter().position(|g| g.id == id))
                else {
                    app.status = "The sampled GPU is no longer available".into();
                    return;
                };
                app.set_view(View::Gpu);
                app.set_gpu_section(0);
                app.selected_gpu = index;
                app.object_scope = Some(context.scope);
            }
            Filter => {
                app.ui_menu = Some(
                    UiMenu::new(
                        "Filter by label",
                        context
                            .tags
                            .iter()
                            .map(|tag| (tag.display(), crossterm::event::KeyCode::Null))
                            .collect(),
                        0,
                    )
                    .with_kind(MenuKind::TagFilter),
                );
                app.tag_menu_context = Some(context);
            }
            Service | Container => {
                if action == Service {
                    let Some((scope, name)) = context.service else {
                        return;
                    };
                    app.set_view(View::Services);
                    app.filter.clear();
                    if let Some(index) = app
                        .filtered_services()
                        .iter()
                        .position(|s| s.scope == scope && s.name == name)
                    {
                        app.selected = index;
                        app.remember_selection();
                    } else {
                        app.status = "Owning service is no longer in the inventory".into();
                    }
                } else {
                    let Some((runtime, id)) = context.container else {
                        return;
                    };
                    app.set_view(View::Containers);
                    app.filter.clear();
                    if let Some(index) = app
                        .filtered_containers()
                        .iter()
                        .position(|c| c.runtime == runtime && c.id == id)
                    {
                        app.selected = index;
                        app.remember_selection();
                    } else {
                        app.status = "Owning container is no longer in the inventory".into();
                    }
                }
            }
            NetworkSettings => task(app, "HOST ROUTES / DNS", self.detail_tx.clone(), || {
                Ok(rstop::network_tools::settings())
            }),
            Storage => app.set_view(View::Storage),
            Test => {
                app.editor.reset();
                app.prompt = context
                    .endpoint
                    .map(|endpoint| ("network target".into(), endpoint, None));
            }
            Logs => {
                app.log_target = context.log_target;
                app.follow_logs = true;
                app.detail_scroll = 0;
                app.detail_column = 0;
                app.detail_filter.clear();
                load_logs(app, self.detail_tx.clone());
            }
            Runtime | Environment | Files | Memory | Cpu => {
                let Some(p) = context.process else {
                    return;
                };
                if !app
                    .snapshot
                    .processes
                    .iter()
                    .any(|live| (live.pid, live.start_time) == (p.pid, p.start_time))
                {
                    app.status = "Process exited or identity changed; reopen the label menu".into();
                    return;
                }
                let title = format!(
                    "{} · PID {}",
                    match action {
                        Runtime => "RUNTIME COMPONENTS",
                        Environment => "COMMUNICATION ENVIRONMENT",
                        Memory => "MEMORY / LIMITS",
                        Cpu => "CPU SCHEDULING",
                        _ => "OPEN FILES",
                    },
                    p.pid
                );
                task(app, &title, self.detail_tx.clone(), move || {
                    if action == Runtime {
                        Ok(rstop::applications::inspect_members(&[p]))
                    } else if matches!(action, Memory | Cpu) {
                        rstop::tags::inspect_resources(&p, action == Memory)
                    } else {
                        rstop::tags::inspect(&p, action == Environment)
                    }
                });
            }
        }
    }
}
