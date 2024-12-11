use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

/// Valid Color Values can be:
/// String, e.g. "white", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html>
/// indexed, e.g. "10", see <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>
/// hex, e.g. "#a03f49", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html#method.deserialize>
/// `toml_example` seems to not handle index and hex well, so the default is pure strings
#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct Theme {
    /// Default Background
    #[toml_example(default = "black")]
    pub background: Color,

    /// Default Text Colour
    #[toml_example(default = "white")]
    pub foreground: Color,

    /// Background for highlighted lines
    #[toml_example(default = "darkgrey")]
    pub background_highlight: Color,

    /// Foreground for highlighted lines
    #[toml_example(default = "white")]
    pub foreground_highlight: Color,

    /// background for unread message highlight
    #[toml_example(default = "lightgray")]
    pub background_unread_message: Color,

    /// Foreground for unread message highlight
    #[toml_example(default = "darkgray")]
    pub foreground_unread_message: Color,

    /// Text Colour for Chat and User table Headers
    #[toml_example(default = "blue")]
    pub table_header: Color,

    /// Text Colour for titlebar contents
    #[toml_example(default = "darkgray")]
    pub foreground_titlebar: Color,

    /// Text Colour for titlebar contents
    #[toml_example(default = "blue")]
    pub background_important_titlebar: Color,

    /// Text Colour for titlebar contents
    #[toml_example(default = "white")]
    pub foreground_important_titlebar: Color,

    /// Foreground for Away Users
    #[toml_example(default = "blue")]
    pub user_away: Color,

    /// Foreground for DND Users
    #[toml_example(default = "red")]
    pub user_dnd: Color,

    /// Foreground for Offline Users
    #[toml_example(default = "gray")]
    pub user_offline: Color,

    /// Foreground for Online Users
    #[toml_example(default = "green")]
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
