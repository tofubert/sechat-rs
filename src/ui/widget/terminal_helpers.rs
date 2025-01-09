use crate::config::Config;
use cfg_if::cfg_if;
use color_eyre::{
    config::{EyreHook, HookBuilder, PanicHook},
    eyre,
};
use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};
use tracing::error;

pub fn install_hooks(config: &Config) -> eyre::Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();

    cfg_if! {
        if #[cfg(debug_assertions)] {
            install_better_panic();
        } else {
            human_panic::setup_panic!();
        }
    }
    install_color_eyre_panic_hook(panic_hook, config);
    install_eyre_hook(eyre_hook, config)?;

    Ok(())
}

#[allow(dead_code)]
fn install_better_panic() {
    better_panic::Settings::auto()
        .most_recent_first(false)
        .verbosity(better_panic::Verbosity::Full)
        .install();
}

fn install_color_eyre_panic_hook(panic_hook: PanicHook, config: &Config) {
    // convert from a `color_eyre::config::PanicHook`` to a `Box<dyn
    // Fn(&PanicInfo<'_>`
    let panic_hook = panic_hook.into_panic_hook();
    let get_enable_mouse = config.get_enable_mouse();
    let get_enable_paste = config.get_enable_paste();
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Err(err) = restore(get_enable_mouse, get_enable_paste) {
            error!("Unable to restore terminal: {err:?}");
        }

        // TODO not sure about this
        // let msg = format!("{}", panic_hook.panic_report(panic_info));
        // error!("Error: {}", strip_ansi_escapes::strip_str(msg));
        panic_hook(panic_info);
    }));
}

fn install_eyre_hook(eyre_hook: EyreHook, config: &Config) -> color_eyre::Result<()> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    let get_enable_mouse = config.get_enable_mouse();
    let get_enable_paste = config.get_enable_paste();
    eyre::set_hook(Box::new(move |error| {
        restore(get_enable_mouse, get_enable_paste).unwrap();
        eyre_hook(error)
    }))?;
    Ok(())
}

pub fn init(
    get_enable_mouse: bool,
    get_enable_paste: bool,
) -> eyre::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    use std::io::stdout;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    if execute!(
        stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_err()
    {
        log::warn!("Consider using a Terminal that supports KeyboardEnhancementFlags.");
    }
    if get_enable_mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    if get_enable_paste {
        execute!(stdout(), EnableBracketedPaste)?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

pub fn restore(get_enable_mouse: bool, get_enable_paste: bool) -> eyre::Result<()> {
    use std::io::stdout;

    if get_enable_paste {
        execute!(stdout(), DisableBracketedPaste)?;
    }
    if get_enable_mouse {
        execute!(stdout(), DisableMouseCapture)?;
    }

    //proceed here regardless of error, since this will fail if the terminal doesn't support this.
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
