use super::Input;
use crossterm::event::{KeyCode, KeyEvent};
use rstop::{
    app::App,
    updates::{self, Phase},
};

impl Input<'_> {
    pub(super) fn update_key(&mut self, app: &mut App, key: KeyEvent) {
        let Some(mut notice) = app.update_notice.take() else {
            return;
        };
        if matches!(notice.phase, Phase::Installing) && key.code != KeyCode::Char('r') {
            app.update_notice = Some(notice);
            return;
        }
        if app.popup_rect.is_empty() {
            if !matches!(key.code, KeyCode::Esc | KeyCode::Char('i')) {
                app.update_notice = Some(notice);
            }
            return;
        }
        if key.code == KeyCode::Char('r') {
            if super::menu::open_url(&notice.release.html_url, self.action_tx.clone()).is_err() {
                app.status = "Browser unavailable; open the release URL above".into();
            }
            app.update_notice = Some(notice);
            return;
        }
        if matches!(notice.phase, Phase::Installed(_)) {
            if !matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('i')) {
                app.update_notice = Some(notice);
            }
            return;
        }
        let count = if matches!(notice.phase, Phase::Failed(_)) {
            2
        } else {
            3
        };
        let action = match key.code {
            KeyCode::Tab | KeyCode::Right | KeyCode::Down => {
                notice.selected = (notice.selected + 1) % count;
                None
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Up => {
                notice.selected = (notice.selected + count - 1) % count;
                None
            }
            KeyCode::Home => {
                notice.selected = 0;
                None
            }
            KeyCode::End => {
                notice.selected = count - 1;
                None
            }
            KeyCode::Enter => Some(notice.selected),
            KeyCode::Char('u') => Some(0),
            KeyCode::Esc | KeyCode::Char('i') => Some(1),
            KeyCode::Char('n') if count == 3 => Some(2),
            _ => None,
        };
        match action {
            Some(0) if !self.read_only => {
                notice.phase = Phase::Installing;
                updates::install_background(notice.release.clone(), self.update_tx.clone());
            }
            Some(0) => {
                notice.phase = Phase::Failed("Updates are disabled in read-only mode".into())
            }
            Some(1) => return,
            Some(2) if !self.read_only => {
                let version = notice.release.version().to_owned();
                let saved = rstop::config::Config::load(self.path).and_then(|mut saved| {
                    saved.ignored_update_version = Some(version.clone());
                    saved.save(self.path)
                });
                if let Err(error) = saved {
                    notice.phase = Phase::Failed(format!("Could not save preference: {error}"));
                    notice.selected = 1;
                } else {
                    app.config.ignored_update_version = Some(version);
                    return;
                }
            }
            Some(2) => {
                notice.phase =
                    Phase::Failed("Saving preferences is disabled in read-only mode".into())
            }
            _ => {}
        }
        app.update_notice = Some(notice);
    }
}
