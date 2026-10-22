use crossterm::event::KeyCode;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Settings,
    Help,
    Quit,
}

impl MenuAction {
    pub const ALL: [Self; 3] = [Self::Settings, Self::Help, Self::Quit];

    pub fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Help => "Help",
            Self::Quit => "Quit",
        }
    }

    pub fn key(self) -> KeyCode {
        match self {
            Self::Settings => KeyCode::F(2),
            Self::Help => KeyCode::F(1),
            Self::Quit => KeyCode::F(10),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectLink {
    Developer,
    Source,
    Coffee,
}

impl ProjectLink {
    pub fn key(self) -> KeyCode {
        KeyCode::Char(match self {
            Self::Developer => 'd',
            Self::Source => 'g',
            Self::Coffee => 'b',
        })
    }

    pub fn url(self) -> &'static str {
        use crate::project;
        match self {
            Self::Developer => project::PROFILE,
            Self::Source => project::SOURCE,
            Self::Coffee => project::COFFEE,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MainMenu {
    pub selected: usize,
    pub hitboxes: Vec<(Rect, usize)>,
    pub links: Vec<(Rect, ProjectLink)>,
    pub back_rect: Rect,
    pub status: String,
    pub backdrop: Option<ratatui::buffer::Buffer>,
    pub backdrop_theme: String,
}
