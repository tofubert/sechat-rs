use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::str::FromStr;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Theme {
    /// Default Background
    #[serde_as(as = "DisplayFromStr")]
    pub background: Color,
    /// Default Text Colour
    #[serde_as(as = "DisplayFromStr")]
    pub foreground: Color,
    #[serde_as(as = "DisplayFromStr")]
    pub background_highlight: Color,
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_highlight: Color,

    /// background for unread message highlight
    #[serde_as(as = "DisplayFromStr")]
    pub background_unread_message: Color,
    /// Foreground for unread message highlight
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_unread_message: Color,

    /// Text Colour for Chat and User table Headers
    #[serde_as(as = "DisplayFromStr")]
    pub table_header: Color,

    /// Text Colour for titlebar contents
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_titlebar: Color,
    /// Text Colour for titlebar contents
    #[serde_as(as = "DisplayFromStr")]
    pub background_important_titlebar: Color,
    /// Text Colour for titlebar contents
    #[serde_as(as = "DisplayFromStr")]
    pub foreground_important_titlebar: Color,

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
        Style::new().fg(self.foreground).bg(self.background)
    }
    pub fn default_highlight_style(&self) -> Style {
        Style::new()
            .fg(self.foreground_highlight)
            .bg(self.background_highlight)
    }
    pub fn user_away_style(&self) -> Style {
        Style::new().fg(self.user_away).bg(self.background)
    }
    pub fn user_dnd_style(&self) -> Style {
        Style::new().fg(self.user_dnd).bg(self.background)
    }
    pub fn user_offline_style(&self) -> Style {
        Style::new().fg(self.user_offline).bg(self.background)
    }
    pub fn user_online_style(&self) -> Style {
        Style::new().fg(self.user_online).bg(self.background)
    }
    pub fn unread_message_style(&self) -> Style {
        Style::new()
            .fg(self.foreground_unread_message)
            .bg(self.background_unread_message)
    }

    pub fn table_header_style(&self) -> Style {
        Style::new()
            .bold()
            .fg(self.table_header)
            .bg(self.background)
    }

    pub fn title_status_style(&self) -> Style {
        Style::new()
            .bg(self.background)
            .fg(self.foreground_titlebar)
    }

    pub fn title_important_style(&self) -> Style {
        Style::new()
            .bold()
            .bg(self.background_important_titlebar)
            .fg(self.foreground_important_titlebar)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::DarkGray,
            foreground: Color::White,
            background_highlight: Color::Gray,
            foreground_highlight: Color::White,
            user_away: Color::Blue,
            user_dnd: Color::Red,
            user_offline: Color::Gray,
            user_online: Color::Green,
            background_unread_message: Color::from_str("#6e6a86").unwrap(),
            foreground_unread_message: Color::from_str("#e0def4").unwrap(),
            table_header: Color::Blue,
            foreground_titlebar: Color::White,
            background_important_titlebar: Color::Red,
            foreground_important_titlebar: Color::White,
        }
    }
}
