mod data;

use color_eyre::eyre::eyre;
use data::Data;
use etcetera::{app_strategy::Xdg, choose_app_strategy, AppStrategy, AppStrategyArgs};
use log::LevelFilter;
use std::{path::PathBuf, process::exit, sync::OnceLock};
use toml_example::TomlExample;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug)]
pub struct Config {
    pub data: Data,
    strategy: Xdg,
}

pub fn init(path_arg: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("Config Path: {:?}", config_path.as_os_str());

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
            std::fs::create_dir_all(config_path.parent().expect("Config File has no Parent"))
                .expect("Could not Create Config dir");
        }
        Data::to_toml_example(config_path.as_os_str().to_str().unwrap()).unwrap();
        println!("Please Update the config with sensible values!");
        exit(0);
    }
    let data = match toml::from_str(&std::fs::read_to_string(config_path).unwrap()) {
        Ok(good_data) => good_data,
        Err(why) => {
            println!("Please Update your config {why} ");
            let example_config_path = config_path_base.join("config.toml_new");
            println!(
                "Writing example config to {}",
                example_config_path
                    .as_os_str()
                    .to_str()
                    .expect("Failed to make config path into string")
            );
            Data::to_toml_example(example_config_path.as_os_str().to_str().unwrap()).unwrap();
            return Err(Box::new(why));
        }
    };

    let mut config = Config::default();
    config.set_config_data(data);
    config.set_strategy(strategy);
    CONFIG
        .set(config)
        .map_err(|config| eyre!("failed to set config {config:?}"))
        .expect("Could not set global config!");
    Ok(())
}

/// Get the application configuration.
///
/// This function should only be called after [`init()`] has been called.
///
/// # Panics
///
/// This function will panic if [`init()`] has not been called.
pub fn get() -> &'static Config {
    CONFIG.get().expect("config not initialized")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data: Data::default(),
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
    pub fn set_config_data(&mut self, data: Data) {
        self.data = data;
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
    use super::*;

    // The ordering of these tests is important since we set the static CONFIG object!

    #[test]
    #[should_panic(expected = "config not initialized")]
    fn get_config_before_init() {
        get();
    }
    #[test]
    #[should_panic(
        expected = "Could not Create Config dir: Os { code: 13, kind: PermissionDenied, message: \"Permission denied\" }"
    )]
    fn init_with_faulty_path() {
        assert!(init("/bogus_test/path").is_err());
    }

    #[test]
    fn default_values() {
        assert!(init("./test/").is_ok());
        assert!(get().get_data_dir().ends_with(".local/share/sechat-rs"));
        assert!(get()
            .get_server_data_dir()
            .ends_with(".local/share/sechat-rs/MyNCInstance"));
        assert!(get()
            .get_http_dump_dir()
            .expect("Not Https Dump Dir found")
            .ends_with(".local/share/sechat-rs"));
        assert!(get().get_enable_mouse());
        assert!(get().get_enable_paste());
    }
    #[test]
    fn init_logging() {
        let conf = Config::default();
        conf.config_logging();
    }

    #[test]
    fn update_data() {
        let mut conf = Config::default();
        conf.set_config_data(Data::default());
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
