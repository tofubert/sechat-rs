use serde::{Deserialize, Serialize};
use toml_example::TomlExample;

#[derive(Serialize, Deserialize, Debug, Default, TomlExample)]
pub struct General {
    /// `General.chat_server_name` is the name used for storage and displaying
    /// UPDATE THIS FIELD
    #[toml_example(default = "MyNCInstance")]
    pub chat_server_name: String,

    /// `General.url` is the base url of the NC instance. Do not append any further parts.
    /// UPDATE THIS FIELD
    #[toml_example(default = "https://butz.com/")]
    pub url: String,

    /// `General.user` is the username. Usually not a email address.
    ///  UPDATE THIS FIELD
    #[toml_example(default = "dummy_user")]
    pub user: String,

    /// `General.app_pw` generated by NC. See <https://butz.com/index.php/settings/user/security>
    ///  UPDATE THIS FIELD
    #[toml_example(default = "foobar-asdasd-asdsf")]
    pub app_pw: String,

    /// `General.log_to_file` should a log file be written into the apps data dir?
    #[toml_example(default = true)]
    pub log_to_file: bool,

    /// `General.dump_failed_requests_to_file` should a log file be written into the apps data dir?
    #[toml_example(default = false)]
    pub dump_failed_requests_to_file: bool,
}
