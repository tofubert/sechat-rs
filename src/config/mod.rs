mod data;
mod theme;

use data::ConfigOptions;
use etcetera::{app_strategy::Xdg, choose_app_strategy, AppStrategy, AppStrategyArgs};
use log::LevelFilter;
use serde::de::DeserializeOwned;
use std::{path::Path, path::PathBuf};
use theme::{options::ColorPalette, Theme};
use toml_example::TomlExample;

#[derive(Debug)]
pub struct Config {
    pub data: ConfigOptions,
    pub theme: Theme,
    strategy: Xdg,
}

pub fn check_config_exists_else_create_new<T: TomlExample>(
    config_path: &Path,
) -> Result<(), String> {
    if !config_path.exists() {
        println!(
            "Config files doesn't exist creating default now at {}.",
            config_path
                .as_os_str()
                .to_str()
                .expect("Failed to make config path into string")
        );
        if !config_path
            .parent()
            .expect("Config Path has no parent")
            .exists()
        {
            let Ok(()) =
                std::fs::create_dir_all(config_path.parent().expect("Config File has no Parent"))
            else {
                return Err("Failed to create Config Dir. Make Sure Dir is creatable.".to_owned());
            };
        }
        T::to_toml_example(config_path.as_os_str().to_str().unwrap()).unwrap();
        println!("Please Update the config with sensible values!");
        return Err("Config File not Present yet!".to_owned());
    }
    Ok(())
}

pub fn read_config_file<T: TomlExample + DeserializeOwned>(
    config_path: &PathBuf,
) -> Result<T, String> {
    let data = match toml::from_str(&std::fs::read_to_string(config_path).unwrap()) {
        Ok(good_data) => good_data,
        Err(why) => {
            println!("Please Update your config {why} ");
            let example_config_path = config_path.join("_new");
            println!(
                "Writing example config to {}",
                example_config_path
                    .as_os_str()
                    .to_str()
                    .expect("Failed to make config path into string")
            );
            T::to_toml_example(example_config_path.as_os_str().to_str().unwrap()).unwrap();
            return Err("Failed to read Config File.".to_owned());
        }
    };
    Ok(data)
}

pub fn init(path_arg: &str) -> Result<Config, String> {
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: "org".to_string(),
        author: "emlix".to_string(),
        app_name: "sechat-rs".to_string(),
    })
    .unwrap();
    let config_path_base = if path_arg.is_empty() {
        strategy.config_dir()
    } else {
        println!(
            "Please consider using the default config file location. {}",
            strategy.config_dir().as_os_str().to_str().unwrap()
        );
        path_arg.into()
    };
    let config_path = config_path_base.join("config.toml");
    let theme_path = config_path_base.join("theme.toml");

    println!("Config Path: {:?}", config_path.as_os_str());

    check_config_exists_else_create_new::<ConfigOptions>(&config_path)?;
    check_config_exists_else_create_new::<ColorPalette>(&theme_path)?;

    let data = read_config_file::<ConfigOptions>(&config_path)?;
    let theme_data = read_config_file::<ColorPalette>(&theme_path)?;

    let mut config = Config::default();
    config.set_config_data(data);
    config.set_theme(theme_data);
    config.set_strategy(strategy);
    Ok(config)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data: ConfigOptions::default(),
            theme: Theme::default(),
            strategy: choose_app_strategy(AppStrategyArgs {
                top_level_domain: "org".to_string(),
                author: "emlix".to_string(),
                app_name: "sechat-rs".to_string(),
            })
            .expect("Could not create default strategy"),
        }
    }
}

impl Config {
    pub fn set_config_data(&mut self, data: ConfigOptions) {
        self.data = data;
    }
    pub fn set_theme(&mut self, data: ColorPalette) {
        self.theme.set_theme(data);
    }
    pub fn set_strategy(&mut self, strategy: Xdg) {
        self.strategy = strategy;
    }
    pub fn get_http_dump_dir(&self) -> Option<PathBuf> {
        if self.data.general.dump_failed_requests_to_file {
            Some(self.get_data_dir())
        } else {
            None
        }
    }

    pub fn get_data_dir(&self) -> PathBuf {
        self.strategy.data_dir()
    }
    pub fn get_server_data_dir(&self) -> PathBuf {
        let path = self
            .strategy
            .data_dir()
            .join(self.data.general.chat_server_name.clone());
        if !path.exists() {
            std::fs::create_dir_all(path.clone()).expect("Failed to create server data path");
        }
        path
    }

    pub fn get_enable_mouse(&self) -> bool {
        self.data.ui.use_mouse
    }

    pub fn get_enable_paste(&self) -> bool {
        self.data.ui.use_paste
    }

    pub fn config_logging(&self) {
        use log4rs::{
            append::{
                console::{ConsoleAppender, Target},
                file::FileAppender,
            },
            config::{Appender, Logger, Root},
            encode::pattern::PatternEncoder,
            filter::threshold::ThresholdFilter,
        };

        let log_path = self.strategy.data_dir().join("app.log");

        // Build a stderr logger.
        let stderr = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{h({l})} {m}{n}")))
            .target(Target::Stderr)
            .build();

        // Logging to log file.
        let log_file = FileAppender::builder()
            // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
            .encoder(Box::new(PatternEncoder::new(
                "{d(%H:%M:%S)} {l} {M}: {m}{n}",
            )))
            .append(false)
            .build(log_path)
            .unwrap();

        // Log Trace level output to file where trace is the default level
        // and the programmatically specified level to stderr.
        let mut config_builder = log4rs::Config::builder()
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Warn)))
                    .build("stderr", Box::new(stderr)),
            )
            .logger(Logger::builder().build("reqwest::connect", LevelFilter::Info));
        let mut root = Root::builder().appender("stderr");
        if self.data.general.log_to_file {
            config_builder =
                config_builder.appender(Appender::builder().build("logfile", Box::new(log_file)));
            root = root.appender("logfile");
        }
        let config = config_builder
            .build(root.build(log::LevelFilter::Debug))
            .unwrap();

        log4rs::init_config(config).expect("Failed to init logging");
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;
    use ratatui::style::Style;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn init_with_faulty_path() {
        let res = init("/bogus_test/path");
        assert_eq!(
            res.err(),
            Some("Failed to create Config Dir. Make Sure Dir is creatable.".to_owned())
        );
    }

    #[test]
    fn init_empty_path() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let res = init("");
        assert_eq!(res.err(), Some("Config File not Present yet!".to_owned()));
    }

    #[test]
    fn init_without_existing_config() {
        let tmp_dir = tempdir().unwrap();
        let res = init(tmp_dir.path().to_str().unwrap());
        assert_eq!(res.err(), Some("Config File not Present yet!".to_owned()));
    }

    #[test]
    fn default_values() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        assert!(config.get_data_dir().ends_with(".local/share/sechat-rs"));
        assert!(config
            .get_server_data_dir()
            .ends_with(".local/share/sechat-rs/MyNCInstance"));
        assert!(config
            .get_http_dump_dir()
            .expect("Not Https Dump Dir found")
            .ends_with(".local/share/sechat-rs"));
        assert!(config.get_enable_mouse());
        assert!(config.get_enable_paste());
    }

    #[test]
    fn default_theme() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        assert_eq!(
            config.theme.default_style(),
            Style::new().fg(Color::White).bg(Color::Black)
        );
    }

    #[test]
    fn init_logging() {
        let conf = Config::default();
        conf.config_logging();
    }

    #[test]
    fn update_data() {
        let mut conf = Config::default();
        conf.set_config_data(ConfigOptions::default());
        conf.set_strategy(
            choose_app_strategy(AppStrategyArgs {
                top_level_domain: "org".to_string(),
                author: "emlix".to_string(),
                app_name: "sechat-rs".to_string(),
            })
            .unwrap(),
        );
        assert!(conf.get_data_dir().ends_with(".local/share/sechat-rs"));
        assert!(conf
            .get_server_data_dir()
            .ends_with(".local/share/sechat-rs"));
        assert!(conf.get_http_dump_dir().is_none());
        assert!(!conf.get_enable_mouse());
        assert!(!conf.get_enable_paste());
    }
}
