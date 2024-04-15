mod general;
mod notifications;
mod ui;

use general::General;
use notifications::Notifications;
use serde::{Deserialize, Serialize};
use toml_example::TomlExample;
use ui::Ui;

#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct Data {
    #[toml_example(nesting)]
    pub general: General,
    #[toml_example(nesting)]
    pub notifications: Notifications,
    #[toml_example(nesting)]
    pub ui: Ui,
}
