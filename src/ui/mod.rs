pub mod app;
pub mod chat_box;
pub mod chat_selector;
pub mod help_box;
pub mod input_box;
pub mod title_bar;
pub mod users;

use super::{
    backend::nc_talk::NCTalk,
    config::{self},
    ui::app::{App, CurrentScreen},
};
use cfg_if::cfg_if;
use color_eyre::{
    config::{EyreHook, HookBuilder, PanicHook},
    eyre::{self, Result},
};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{
    error::Error,
    io::{stdout, Stdout},
    panic,
    time::Duration,
};
use tracing::error;

pub fn install_hooks() -> Result<()> {
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
    install_color_eyre_panic_hook(panic_hook);
    install_eyre_hook(eyre_hook)?;

    Ok(())
}

#[allow(dead_code)]
fn install_better_panic() {
    better_panic::Settings::auto()
        .most_recent_first(false)
        .verbosity(better_panic::Verbosity::Full)
        .install();
}

fn install_color_eyre_panic_hook(panic_hook: PanicHook) {
    // convert from a `color_eyre::config::PanicHook`` to a `Box<dyn
    // Fn(&PanicInfo<'_>`
    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Err(err) = restore() {
            error!("Unable to restore terminal: {err:?}");
        }

        // not sure about this
        // let msg = format!("{}", panic_hook.panic_report(panic_info));
        // error!("Error: {}", strip_ansi_escapes::strip_str(msg));
        panic_hook(panic_info);
    }));
}

fn install_eyre_hook(eyre_hook: EyreHook) -> color_eyre::Result<()> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        restore().unwrap();
        eyre_hook(error)
    }))?;
    Ok(())
}

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    if config::get().get_enable_mouse() {
        execute!(stdout(), EnableMouseCapture)?;
    }
    if config::get().get_enable_paste() {
        execute!(stdout(), EnableBracketedPaste)?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    if config::get().get_enable_paste() {
        execute!(stdout(), DisableBracketedPaste)?;
    }
    if config::get().get_enable_mouse() {
        execute!(stdout(), DisableMouseCapture)?;
    }
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

enum ProcessEventResult {
    Continue,
    Exit,
}

pub async fn run(nc_backend: NCTalk) -> Result<(), Box<dyn Error>> {
    install_hooks()?;

    // create app and run it
    run_app(
        init().expect("Failed to init Terminal UI."),
        app::App::new(nc_backend),
    )
    .await?;

    restore()?;
    Ok(())
}

async fn run_app<B: Backend>(
    mut terminal: Terminal<B>,
    mut app: App<'_>,
) -> Result<(), Box<dyn Error>> {
    app.select_room().await?;
    log::debug!("Entering Main Loop");
    loop {
        terminal.draw(|f| app.ui(f))?;

        // Event within timeout?
        if event::poll(Duration::from_millis(3000))? {
            match process_event(&mut app, event::read()?).await {
                Ok(ProcessEventResult::Continue) => (),
                Ok(ProcessEventResult::Exit) => return Ok(()),
                Err(why) => return Err(why),
            }
        } else {
            log::debug!("Looking for Updates on the server.");
            // trigger a fetch from upstream for messages
            app.fetch_updates().await?;
        }
    }
}

async fn process_event(
    app: &mut App<'_>,
    event: Event,
) -> Result<ProcessEventResult, Box<dyn Error>> {
    // It's guaranteed that `read` won't block, because `poll` returned
    // `Ok(true)`.
    match event {
        Event::Key(key) if key.kind != event::KeyEventKind::Release => {
            log::debug!("Processing key event {:?}", key.code);
            match app.current_screen {
                CurrentScreen::Helping => handle_key_in_help(key, app),
                CurrentScreen::Reading => handle_key_in_reading(key, app).await?,
                CurrentScreen::Exiting => {
                    if let Some(value) = handle_key_in_exit(key, app) {
                        return value;
                    }
                }
                CurrentScreen::Editing => handle_key_in_editing(key, app).await?,
                CurrentScreen::Opening => handle_key_in_opening(key, app).await?,
            }
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollDown => app.scroll_down(),
            MouseEventKind::ScrollUp => app.scroll_up(),
            MouseEventKind::Down(_button) => {
                app.click_at(Position::new(mouse.column, mouse.row))?;
            }
            _ => (),
        },
        _ => (),
    }
    Ok(ProcessEventResult::Continue)
}

async fn handle_key_in_opening(
    key: event::KeyEvent,
    app: &mut App<'_>,
) -> Result<(), Box<dyn Error>> {
    match key.code {
        KeyCode::Esc => app.current_screen = CurrentScreen::Reading,
        KeyCode::Char('h') | KeyCode::Left => _ = app.selector.state.key_left(),
        KeyCode::Char('j') | KeyCode::Down => _ = app.selector.state.key_down(),
        KeyCode::Char('k') | KeyCode::Up => _ = app.selector.state.key_up(),
        KeyCode::Char('l') | KeyCode::Right => _ = app.selector.state.key_right(),
        KeyCode::Char('q') => app.current_screen = CurrentScreen::Exiting,
        KeyCode::Char('?') => app.current_screen = CurrentScreen::Helping,
        KeyCode::Char(' ') => _ = app.selector.state.toggle_selected(),
        KeyCode::Enter => app.select_room().await?,
        KeyCode::Home => _ = app.selector.state.select_first(),
        KeyCode::End => _ = app.selector.state.select_last(),
        KeyCode::PageDown => _ = app.selector.state.scroll_down(3),
        KeyCode::PageUp => _ = app.selector.state.scroll_up(3),
        _ => (),
    };
    Ok(())
}

async fn handle_key_in_editing(
    key: event::KeyEvent,
    app: &mut App<'_>,
) -> Result<(), Box<dyn Error>> {
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Enter => {
                // SEND MEssage
                app.current_screen = CurrentScreen::Reading;
                app.send_message().await?;
            }
            KeyCode::Backspace => app.pop_input(),
            KeyCode::Esc => app.current_screen = CurrentScreen::Reading,
            KeyCode::Char(value) => app.append_input(value),
            _ => (),
        };
    }

    Ok(())
}

fn handle_key_in_help(key: event::KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') => app.current_screen = CurrentScreen::Exiting,
        KeyCode::Esc => app.current_screen = CurrentScreen::Reading,
        KeyCode::Char('o') => app.current_screen = CurrentScreen::Opening,
        _ => (),
    }
}

fn handle_key_in_exit(
    key: event::KeyEvent,
    app: &mut App,
) -> Option<Result<ProcessEventResult, Box<dyn Error>>> {
    match key.code {
        KeyCode::Char('?') => app.current_screen = CurrentScreen::Helping,
        KeyCode::Char('y') => {
            if let Err(err) = app.write_log_files() {
                log::warn!(
                    "Failure to store logs into log file ({}), ignoring for now.",
                    err
                );
            }
            return Some(Ok(ProcessEventResult::Exit));
        }
        KeyCode::Char('n') => app.current_screen = CurrentScreen::Reading,
        _ => (),
    }
    None
}

async fn handle_key_in_reading(
    key: event::KeyEvent,
    app: &mut App<'_>,
) -> Result<(), Box<dyn Error>> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_screen = CurrentScreen::Exiting;
        }
        KeyCode::Char('e' | 'i') => app.current_screen = CurrentScreen::Editing,
        KeyCode::Char('j') | KeyCode::Down if key.kind == KeyEventKind::Press => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up if key.kind == KeyEventKind::Press => app.scroll_up(),
        KeyCode::Char('m') => app.mark_current_as_read().await?,
        KeyCode::Char('o') => app.current_screen = CurrentScreen::Opening,
        KeyCode::Char('q') => app.current_screen = CurrentScreen::Exiting,
        KeyCode::Char('?') => app.current_screen = CurrentScreen::Helping,
        KeyCode::Char('u') => app.toggle_user_sidebar(),
        _ => (),
    };
    Ok(())
}
