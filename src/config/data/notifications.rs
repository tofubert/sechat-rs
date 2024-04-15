use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct Notifications {
    /// `Notifications.timout_ms` how long a notification shall be displayed.
    #[toml_example(default = 5000)]
    pub timeout_ms: u32,
    #[toml_example(default = false)]
    pub persistent: bool,
    #[toml_example(default = false)]
    pub silent: bool,
}
