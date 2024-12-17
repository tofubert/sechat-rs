#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![doc(issue_tracker_base_url = "https://github.com/tofubert/sechat-rs/issues")]
//! <div align="center">
//!
//! [![Crate Badge]][Crate] [![Docs Badge]][API Docs] [![CI Badge]][CI Workflow] [![Deps.rs
//! Badge]][Deps.rs]<br> [![Codecov Badge]][Codecov] [![License Badge]](./LICENSE)
//!
//! [Report a bug] · [Request a Feature] · [Create a Pull Request]
//!
//! </div>
//!
//! # NC Talk TUI Client
//!
//! Sechat-rs is a Event driven TUI for Nextcloud Talk, based on [Ratatui].
//! It uses the [Nextcloud Talk API] to communicate with a NC Server.
//!
//!
//! ## Building
//! This Project uses [cargo-make] to build and execute the different checkers.
//! Calling ```cargo make all``` is the simplest way to run all needed targets for the ci to be happy as well.
//! You might need to add the [cargo-make] crate to your system before you can build.
//!
//! Check the [Makefile] for all build targets.
//! No ```run``` make target was created since it would be a duplicate to ```cargo run```.
//!
//! ## Usage
//!
//! You can call the client without any arguments and it will search the default XDG path for a config:
//! ```
//! cargo r
//! ```
//! Or you can pass a config file path directly. Note: The Config Arg takes a path to the folder containing the config. It expects a ```config.toml``` and a ```theme.toml``` there.
//! ``` bash
//! cargo r -- -c ~/.config/sechat-rs/
//! ```
//!
//! ## Design Considerations
//!
//! The Client is split into 3 main parts.
//! - [`Backend`] for Abstracting the NC API and Storing the NC Talk data.
//! - [`Config`] for fetching,writing and creating the config files on disk.
//! - [`UI`] for rendeing the UI and sending/controling requests to the backend.
//!
//! ### Testing and Mocking
//! For the Testing sechat-rs makes heavy use of the [`mockall`] crate. This forces the declaration of a trait type for all of the [`Backend`] structs.
//! This makes the Code for the Backend somewhat harder to read than it needs to be, but is needed in order to achieve a testability.
//!
//! ### Config
//! On Startup the ```config.toml``` and ```theme.toml``` are attempted to be read by [`config::init`].
//! If no config is found a default config is written to the location and the user is asked to fill it with the needed info.
//! If parsing of a config fails, a default config is put next to the failed one with a ```_new``` postfix.
//!
//! ### Backend
//! With the config read the [`backend::nc_talk::NCTalk`] is created, which first tries to read old chat logs from disk, and the fetches updates from the server.
//! Note: Currently gaps in the chat history are not filled automatically.
//! The `NCTalk` instance is holding a list of all [`backend::nc_room::NCRoom`]s.
//! [`backend::nc_request::NCRequest`] is the actual API Requester, which will call to the server and Parse the Response Objects.
//! Responses are parsed using [`serde`] and various structs in [`backend::nc_request`].
//!
//! ### UI
//! The backend is then passed into [`ui::run`] which starts the UI and holds the main event loop.
//! Ether a [`crossterm::event::KeyEvent`] or the rendering timeout lead to a refresh of the UI.
//! The UI uses the [`ui::app::App`] struct to orchestrate all the UIs Widgets.
//!
//!
//!
//! [Report a Bug]: https://github.com/tofubert/sechat-rs/issues/new?labels=bug
//! [Request a Feature]: https://github.com/tofubert/sechat-rs/issues/new?labels=enhancement
//! [Create a Pull Request]: https://github.com/tofubert/sechat-rs/compare
//! [API Docs]: https://docs.rs/sechat-rs
//! [Nextcloud Talk API]: https://nextcloud-talk.readthedocs.io/en/stable/
//! [Makefile]: https://github.com/tofubert/sechat-rs/blob/main/Makefile.toml
//! [Ratatui]: `ratatui`
//! [`Backend`]: backend
//! [`Config`]: config
//! [`UI`]: ui
//! [`mockall`]: https://crates.io/crates/mockall
//! [cargo-make]: https://crates.io/crates/cargo-make
//! [Crate]: https://crates.io/crates/sechat-rs
//! [Crate Badge]: https://img.shields.io/crates/v/sechat-rs?logo=rust&style=flat-square&logoColor=E05D44&color=E05D44
//! [License Badge]: https://img.shields.io/crates/l/sechat-rs?style=flat-square&color=1370D3
//! [CI Badge]: https://img.shields.io/github/actions/workflow/status/tofubert/sechat-rs/rust.yml?style=flat-square&logo=github
//! [CI Workflow]: https://github.com/tofubert/sechat-rs/actions/workflows/rust.yml
//! [Codecov Badge]: https://img.shields.io/codecov/c/github/tofubert/sechat-rs?logo=codecov&style=flat-square&token=BAQ8SOKEST&color=C43AC3&logoColor=C43AC3
//! [Codecov]: https://app.codecov.io/gh/tofubert/sechat-rs
//! [Deps.rs Badge]: https://deps.rs/repo/github/tofubert/sechat-rs/status.svg?style=flat-square
//! [Deps.rs]: https://deps.rs/repo/github/tofubert/sechat-rs
//! [Docs Badge]: https://img.shields.io/docsrs/ratatui?logo=rust&style=flat-square&logoColor=E05D44

mod backend;
/// Config and Theme Module
mod config;
// TUI and Event handling module
mod ui;

use clap::Parser;

/// Argument struct for CLI Args. Using the [`clap`] crate.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the configuration File, if omitted will try default config paths.
    /// Default XDG based path is generally encouraged.
    #[arg(short, long, value_name = "PATH", default_value = "")]
    config_path: String,
}

/// Reads Console [`Args`] and [`config`].
/// Creates Backend.
/// Passes Backend into Frontend.
/// Frontend runs infinite loop.
#[tokio::main]
async fn main() {
    let args = Args::parse();

    config::init(&args.config_path).expect("Config init aborted.");
    config::get().config_logging();

    // check if crate has alpha suffix in version
    let pre = env!("CARGO_PKG_VERSION_PRE");
    if !pre.is_empty() {
        log::warn!("Entering Sechat-rs, please be aware this is {pre} SW!");
    }

    // Create API Wrapper for NC Talk API.
    let requester = backend::nc_request::NCRequest::new().expect("cannot create NCRequest");

    // Create Backend, UI and enter UI loop.
    match backend::nc_talk::NCTalk::new(requester).await {
        Ok(backend) => ui::run(backend).await.expect("crashed"),
        Err(why) => {
            log::error!("Failed to create backend because: {why}");
        }
    };
}
