use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

/// Valid Color Values can be:
/// String, e.g. "white", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html>
/// indexed, e.g. "10", see <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>
/// hex, e.g. "#a03f49", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html#method.deserialize>
#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct ColorPalette {
    /// Default Background
    #[toml_example(default = "#1f2335")]
    pub background: Color,

    /// Default Text Colour
    #[toml_example(default = "#c0caf5")]
    pub foreground: Color,

    /// Background for highlighted lines
    #[toml_example(default = "#3b4261")]
    pub background_highlight: Color,

    /// Foreground for highlighted lines
    #[toml_example(default = "#ffc777")]
    pub foreground_highlight: Color,

    /// background for unread message highlight
    #[toml_example(default = "#292e42")]
    pub background_unread_message: Color,

    /// Foreground for unread message highlight
    #[toml_example(default = "#9d7cd8")]
    pub foreground_unread_message: Color,

    /// Text Colour for Chat and User table Headers
    #[toml_example(default = "#394b70")]
    pub table_header: Color,

    /// Text Colour for titlebar contents
    #[toml_example(default = "#545c7e")]
    pub foreground_titlebar: Color,

    /// background for titlebar highlight
    #[toml_example(default = "#292e42")]
    pub background_important_titlebar: Color,

    /// Text Colour for titlebar highlight
    #[toml_example(default = "#9d7cd8")]
    pub foreground_important_titlebar: Color,

    /// Foreground for Away Users
    #[toml_example(default = "#ff9e64")]
    pub user_away: Color,

    /// Foreground for DND Users
    #[toml_example(default = "#c53b53")]
    pub user_dnd: Color,

    /// Foreground for Offline Users
    #[toml_example(default = "#737aa2")]
    pub user_offline: Color,

    /// Foreground for Online Users
    #[toml_example(default = "#c3e88d")]
    pub user_online: Color,

    /// Borders for popup windows
    #[toml_example(default = "#ff757f")]
    pub popup_border: Color,
}
