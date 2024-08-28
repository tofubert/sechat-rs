mod backend;
mod config;
mod ui;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the configuration File, if omitted will try default config paths.
    /// Default XDG based path is generally encouraged.
    #[arg(short, long, value_name = "PATH", default_value = "")]
    config_path: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    config::init(&args.config_path);
    config::get().config_logging();

    // check if crate has alpha suffix in version
    let pre = env!("CARGO_PKG_VERSION_PRE");
    if !pre.is_empty() {
        log::warn!("Entering Seshat-rs, please be aware this is {pre} SW!");
    }

    let requester = backend::nc_request::NCRequest::new().expect("cannot create NCRequest");

    match backend::nc_talk::NCTalk::new(requester).await {
        Ok(backend) => ui::run(backend).await.expect("crashed"),
        Err(why) => {
            log::error!("Failed to create backend because: {why}");
        }
    };
}
