use ratatui::{Terminal, backend::TestBackend};
use rstop::{
    app::App,
    config::Config,
    ui,
    updates::{Notice, Phase, Release},
};

#[test]
fn update_states_keep_their_actions_readable_in_small_terminals() {
    for width in [60, 84, 140] {
        for (phase, required, forbidden) in [
            (Phase::Available, "Upgrade", "Retry"),
            (Phase::Installing, "Upgrading", "Upgrade]"),
            (
                Phase::Installed("0.1.2".into()),
                "Continue monitoring",
                "Retry",
            ),
            (
                Phase::Failed("Network unavailable".into()),
                "Retry",
                "Skip version",
            ),
        ] {
            let mut app = App::new(Config::default());
            let mut notice = Notice::new(Release {
                tag_name: "v0.1.2".into(),
                html_url: "https://github.com/fan-ziqi/rstop/releases/tag/v0.1.2".into(),
                ..Default::default()
            });
            notice.phase = phase;
            app.update_notice = Some(notice);
            let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
            terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let mut text = String::new();
            for y in app.popup_rect.y..app.popup_rect.bottom() {
                for x in app.popup_rect.x..app.popup_rect.right() {
                    text.push_str(buffer[(x, y)].symbol());
                }
                text.push('\n');
            }
            assert!(text.contains("rstop update"), "{width}: {text}");
            assert!(text.contains(required), "{width}: {text}");
            assert!(!text.contains(forbidden), "{width}: {text}");
            assert!(!text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
            for (rect, _) in &app.update_buttons {
                assert!(app.popup_rect.contains((rect.x, rect.y).into()));
            }
        }
    }
}
