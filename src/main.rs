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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = config::init(&args.config_path)?;
    config.config_logging();

    // check if crate has alpha suffix in version
    let pre = env!("CARGO_PKG_VERSION_PRE");
    if !pre.is_empty() {
        log::warn!("Entering Sechat-rs, please be aware this is {pre} SW!");
    }

    let requester = backend::nc_request::NCRequest::new(&config).expect("cannot create NCRequest");

    let backend = match backend::nc_talk::NCTalk::new(requester, &config).await {
        Ok(backend) => backend,
        Err(why) => {
            panic!("Failed to create backend because: {}", why);
        }
    };
    let mut ui: ui::app::App<'_, _> = ui::app::App::new(backend, &config);

    ui.run(&config).await
}
