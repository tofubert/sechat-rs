use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::str::FromStr;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Theme {
    /// Default Backround
    #[serde_as(as = "DisplayFromStr")]
    pub backround: Color,
    /// Default Text Colour
    #[serde_as(as = "DisplayFromStr")]
    pub foreground: Color,

    /// Backround for unread message highlight
    #[serde_as(as = "DisplayFromStr")]
    pub backround_unread_message: Color,
    /// Foreground for unread message highlight
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_unread_message: Color,

    /// Text Colour for Chat and User table Headers
    #[serde_as(as = "DisplayFromStr")]
    pub table_header: Color,

    /// Text Colour for titlebar contents
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_titlebar: Color,

    #[serde_as(as = "DisplayFromStr")]
    pub user_away: Color,
    #[serde_as(as = "DisplayFromStr")]
    pub user_dnd: Color,
    #[serde_as(as = "DisplayFromStr")]
    pub user_offline: Color,
    #[serde_as(as = "DisplayFromStr")]
    pub user_online: Color,
}

impl Theme {
    pub fn default_style(&self) -> Style {
        Style::new().fg(self.foreground).bg(self.backround)
    }
    pub fn user_away_style(&self) -> Style {
        Style::new().fg(self.user_away).bg(self.backround)
    }
    pub fn user_dnd_style(&self) -> Style {
        Style::new().fg(self.user_dnd).bg(self.backround)
    }
    pub fn user_offline_style(&self) -> Style {
        Style::new().fg(self.user_offline).bg(self.backround)
    }
    pub fn user_online_style(&self) -> Style {
        Style::new().fg(self.user_online).bg(self.backround)
    }
    pub fn unread_message_style(&self) -> Style {
        Style::new()
            .fg(self.foreground_unread_message)
            .bg(self.backround_unread_message)
    }

    pub fn table_header_style(&self) -> Style {
        Style::new().bold().fg(self.table_header).bg(self.backround)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            backround: Color::Black,
            foreground: Color::White,
            user_away: Color::Blue,
            user_dnd: Color::Red,
            user_offline: Color::Gray,
            user_online: Color::Green,
            backround_unread_message: Color::from_str("#6e6a86").unwrap(),
            foreground_unread_message: Color::from_str("#e0def4").unwrap(),
            table_header: Color::from_str("#e0def4").unwrap(),
            foreground_titlebar: Color::White,
        }
    }
}
