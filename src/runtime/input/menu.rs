use super::Input;
use crossterm::event::{KeyCode, KeyEvent};
use rstop::app::{
    App, UiMenu,
    menu::{MenuAction, ProjectLink},
};
use std::process::{Command, Stdio};

impl Input<'_> {
    pub(super) fn main_menu_key(&mut self, app: &mut App, key: KeyEvent) -> bool {
        let menu = app.main_menu.as_mut().expect("main menu is open");
        let actions = MenuAction::ALL;
        match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Esc | KeyCode::Char('?') => {
                app.main_menu = None;
                return true;
            }
            KeyCode::Up | KeyCode::Left | KeyCode::BackTab | KeyCode::Char('k') => {
                menu.selected = (menu.selected + actions.len() - 1) % actions.len();
                return true;
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Tab | KeyCode::Char('j') => {
                menu.selected = (menu.selected + 1) % actions.len();
                return true;
            }
            KeyCode::Home => {
                menu.selected = 0;
                return true;
            }
            KeyCode::End => {
                menu.selected = actions.len() - 1;
                return true;
            }
            KeyCode::Char(':') => {
                app.ui_menu = Some(UiMenu::new("Commands", app.commands(), 0));
                return true;
            }
            _ => {}
        }
        if let Some(link) = [
            ProjectLink::Developer,
            ProjectLink::Source,
            ProjectLink::Coffee,
        ]
        .into_iter()
        .find(|link| link.key() == key.code)
        {
            // All URLs come from fixed project metadata; no shell or user input.
            menu.status = match open_url(link.url(), self.action_tx.clone()) {
                Ok(()) => "Opening browser…".into(),
                Err(_) => "Browser unavailable; open the URL above".into(),
            };
            return true;
        }
        let action = if key.code == KeyCode::Enter {
            actions.get(menu.selected).copied()
        } else {
            actions.into_iter().find(|action| action.key() == key.code)
        };
        match action {
            Some(MenuAction::Settings) => app.settings = true,
            Some(MenuAction::Help) => {
                app.help = true;
                app.detail_scroll = 0;
            }
            Some(MenuAction::Quit) => return false,
            None => {}
        }
        true
    }
}

fn open_url(url: &'static str, result: std::sync::mpsc::Sender<String>) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let message = match child.wait() {
            Ok(status) if status.success() => "Browser accepted the URL",
            _ => "Browser unavailable; open the URL above",
        };
        let _ = result.send(message.into());
    });
    Ok(())
}
