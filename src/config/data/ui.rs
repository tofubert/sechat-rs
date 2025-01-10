use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct Ui {
    /// The default room you want to see on startup.
    ///  UPDATE THIS FIELD
    #[toml_example(default = "General")]
    pub default_room: String,
    pub categories: Vec<String>,
    pub categories_separator: String,
    /// Should the userlist be shown in rooms by default?
    #[toml_example(default = true)]
    pub user_sidebar_default: bool,
    #[toml_example(default = true)]
    pub use_mouse: bool,
    #[toml_example(default = true)]
    pub use_paste: bool,
    /// Default is dark-theme. light-theme is also possible
    #[toml_example(default = "dark-theme")]
    pub theme: String,
}
