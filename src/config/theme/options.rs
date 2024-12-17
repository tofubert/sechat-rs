use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

/// Valid Color Values can be:
/// String, e.g. "white", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html>
/// indexed, e.g. "10", see <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>
/// hex, e.g. "#a03f49", see <https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html#method.deserialize>
/// `toml_example` seems to not handle index and hex well, so the default is pure strings
#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct ColorPalette {
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
