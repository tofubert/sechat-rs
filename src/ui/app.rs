use crate::{
    backend::{nc_room::NCRoomInterface, nc_talk::NCBackend},
    config::Config,
    ui::{
        chat_box::ChatBox, chat_selector::ChatSelector, help_box::HelpBox, input_box::InputBox,
        title_bar::TitleBar, users::Users,
    },
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Position},
    style::{Style, Stylize},
    widgets::Paragraph,
    Frame,
};
use ratatui::{prelude::CrosstermBackend, Terminal};
use strum_macros::Display;
use tui_textarea::Input;

use cfg_if::cfg_if;
use color_eyre::{
    config::{EyreHook, HookBuilder, PanicHook},
    eyre,
};
use crossterm::{
    event::{
        poll, read, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
        EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
        KeyboardEnhancementFlags, MouseEventKind, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tracing::error;
use tui_textarea::Key;

enum ProcessEventResult {
    Continue,
    Exit,
}

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

fn init(
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

fn restore(get_enable_mouse: bool, get_enable_paste: bool) -> eyre::Result<()> {
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

#[derive(PartialEq, Clone, Copy, Display)]
pub enum CurrentScreen {
    Reading,
    Opening,
    Editing,
    Exiting,
    Helping,
}

pub struct App<'a, Backend: NCBackend> {
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    backend: Backend,
    title: TitleBar<'a>,
    chat: ChatBox<'a>,
    pub selector: ChatSelector<'a>,
    input: InputBox<'a>,
    help: HelpBox,
    users: Users<'a>,
    user_sidebar_visible: bool,
    default_style: Style,
}

impl<Backend: NCBackend> App<'_, Backend> {
    pub fn new(backend: Backend, config: &Config) -> Self {
        Self {
            current_screen: CurrentScreen::Reading,
            title: TitleBar::new(
                CurrentScreen::Reading,
                backend.get_current_room().to_string(),
                config,
            ),
            selector: ChatSelector::new(&backend, config),
            input: InputBox::new("", config),
            chat: {
                let mut chat = ChatBox::new(config);
                chat.update_messages(&backend);
                chat.select_last_message();
                chat
            },
            users: {
                let mut users = Users::new(config);
                users.update(&backend);
                users
            },
            backend,
            help: HelpBox::new(config),
            user_sidebar_visible: config.data.ui.user_sidebar_default,
            default_style: config.theme.default_style(),
        }
    }

    pub async fn run(&mut self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        install_hooks(config)?;

        let tui = init(config.get_enable_mouse(), config.get_enable_paste())
            .expect("Could not Create TUI Backend.");

        // create app and run it
        self.run_app(tui).await?;

        restore(config.get_enable_mouse(), config.get_enable_paste())?;
        Ok(())
    }
    pub fn ui(&mut self, f: &mut Frame) {
        let base_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(f.area());

        if self.current_screen == CurrentScreen::Opening {
            self.selector.render_area(f, base_layout[1]);
        } else if self.current_screen == CurrentScreen::Exiting {
            f.render_widget(
                Paragraph::new("To Quit Press 'y', to stay 'n'")
                    .alignment(Alignment::Center)
                    .style(self.default_style.bold()),
                base_layout[1],
            );
        } else if self.current_screen == CurrentScreen::Helping {
            self.help.render_area(f, base_layout[1]);
        } else {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(4), Constraint::Length(3)])
                .split(base_layout[1]);

            if self.user_sidebar_visible && self.backend.get_current_room().is_group() {
                let chat_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                    .split(main_layout[0]);
                self.chat
                    .set_width_and_update_if_change(chat_layout[0].width, &self.backend);
                self.chat.render_area(f, chat_layout[0]);
                self.users.render_area(f, chat_layout[1]);
            } else {
                self.chat
                    .set_width_and_update_if_change(main_layout[0].width, &self.backend);
                self.chat.render_area(f, main_layout[0]);
            };

            self.input.render_area(f, main_layout[1]);
        }
        self.title.update(self.current_screen, &self.backend);
        self.title.render_area(f, base_layout[0]);
    }

    pub async fn mark_current_as_read(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.get_current_room().mark_as_read().await?;
        self.backend.update_rooms(true).await?;
        self.update_ui()?;
        Ok(())
    }

    fn update_ui(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.title.update(self.current_screen, &self.backend);
        self.selector.update(&self.backend)?;
        self.chat.update_messages(&self.backend);
        self.users.update(&self.backend);
        Ok(())
    }

    pub async fn send_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.input.is_empty() {
            Ok(())
        } else {
            self.backend
                .send_message(self.input.lines().join("\n"))
                .await?;
            self.input.select_all();
            self.input.cut();
            self.input.select_all();
            self.update_ui()?;
            self.chat.select_last_message();
            Ok(())
        }
    }

    pub async fn select_room(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.selector.state.selected().len() == 2 {
            self.backend
                .select_room(
                    self.selector
                        .state
                        .selected()
                        .last()
                        .expect("no selection available")
                        .clone(),
                )
                .await?;
            self.current_screen = CurrentScreen::Reading;
            self.update_ui()?;
            self.chat.select_last_message();
        } else {
            self.selector.state.toggle_selected();
        }
        Ok(())
    }

    pub async fn fetch_updates(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.update_rooms(false).await?;
        self.update_ui()?;
        Ok(())
    }

    pub fn new_input_key(&mut self, key: Input) {
        self.input.input(key);
    }

    pub fn scroll_up(&mut self) {
        self.chat.select_up();
    }

    pub fn scroll_down(&mut self) {
        self.chat.select_down();
    }

    pub fn toggle_user_sidebar(&mut self) {
        self.user_sidebar_visible = !self.user_sidebar_visible;
    }

    pub fn click_at(&mut self, position: Position) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            CurrentScreen::Reading => self.chat.select_line(position)?,
            CurrentScreen::Opening => {
                self.selector.state.click_at(position);
            }
            _ => (),
        }
        Ok(())
    }

    pub fn write_log_files(&mut self) -> Result<(), std::io::Error> {
        self.backend.write_to_log()
    }

    async fn run_app<B: ratatui::prelude::Backend>(
        &mut self,
        mut terminal: Terminal<B>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.select_room().await?;
        log::debug!("Entering Main Loop");
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Event within timeout?
            if poll(std::time::Duration::from_millis(3000))? {
                match self.process_event(read()?).await {
                    Ok(ProcessEventResult::Continue) => (),
                    Ok(ProcessEventResult::Exit) => return Ok(()),
                    Err(why) => return Err(why),
                }
            } else {
                log::debug!("Looking for Updates on the server.");
                // trigger a fetch from upstream for messages
                self.fetch_updates().await?;
            }
        }
    }

    async fn process_event(
        &mut self,
        event: Event,
    ) -> Result<ProcessEventResult, Box<dyn std::error::Error>> {
        // It's guaranteed that `read` won't block, because `poll` returned
        // `Ok(true)`.
        match event {
            Event::Key(key) => {
                log::debug!("Processing key event {:?}", key);
                match self.current_screen {
                    CurrentScreen::Helping => self.handle_key_in_help(key),
                    CurrentScreen::Reading => self.handle_key_in_reading(key).await?,
                    CurrentScreen::Exiting => {
                        if let Some(value) = self.handle_key_in_exit(key) {
                            return value;
                        }
                    }
                    CurrentScreen::Editing => {
                        self.handle_key_in_editing(Input::from(event.clone()))
                            .await?;
                    }
                    CurrentScreen::Opening => self.handle_key_in_opening(key).await?,
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => self.scroll_down(),
                MouseEventKind::ScrollUp => self.scroll_up(),
                MouseEventKind::Down(_button) => {
                    self.click_at(Position::new(mouse.column, mouse.row))?;
                }
                _ => (),
            },
            _ => {
                log::debug!("Unknown Event {:?}", event);
            }
        }
        Ok(ProcessEventResult::Continue)
    }

    async fn handle_key_in_opening(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Esc => self.current_screen = CurrentScreen::Reading,
            KeyCode::Char('h') | KeyCode::Left => _ = self.selector.state.key_left(),
            KeyCode::Char('j') | KeyCode::Down => _ = self.selector.state.key_down(),
            KeyCode::Char('k') | KeyCode::Up => _ = self.selector.state.key_up(),
            KeyCode::Char('l') | KeyCode::Right => _ = self.selector.state.key_right(),
            KeyCode::Char('q') => self.current_screen = CurrentScreen::Exiting,
            KeyCode::Char('?') => self.current_screen = CurrentScreen::Helping,
            KeyCode::Char(' ') => _ = self.selector.state.toggle_selected(),
            KeyCode::Enter => self.select_room().await?,
            KeyCode::Home => _ = self.selector.state.select_first(),
            KeyCode::End => _ = self.selector.state.select_last(),
            KeyCode::PageDown => _ = self.selector.state.scroll_down(3),
            KeyCode::PageUp => _ = self.selector.state.scroll_up(3),
            _ => (),
        };
        Ok(())
    }

    async fn handle_key_in_editing(
        &mut self,
        key: Input,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            Input { key: Key::Esc, .. } => self.current_screen = CurrentScreen::Reading,
            Input {
                key: Key::Enter,
                shift: false,
                ..
            } => {
                // SEND MEssage
                self.current_screen = CurrentScreen::Reading;
                self.send_message().await?;
            }
            _ => self.new_input_key(key),
        };

        Ok(())
    }

    fn handle_key_in_help(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.current_screen = CurrentScreen::Exiting,
            KeyCode::Esc => self.current_screen = CurrentScreen::Reading,
            KeyCode::Char('o') => self.current_screen = CurrentScreen::Opening,
            _ => (),
        }
    }

    fn handle_key_in_exit(
        &mut self,
        key: KeyEvent,
    ) -> Option<Result<ProcessEventResult, Box<dyn std::error::Error>>> {
        match key.code {
            KeyCode::Char('?') => self.current_screen = CurrentScreen::Helping,
            KeyCode::Char('y') => {
                if let Err(err) = self.write_log_files() {
                    log::warn!(
                        "Failure to store logs into log file ({}), ignoring for now.",
                        err
                    );
                }
                return Some(Ok(ProcessEventResult::Exit));
            }
            KeyCode::Char('n') => self.current_screen = CurrentScreen::Reading,
            _ => (),
        }
        None
    }

    async fn handle_key_in_reading(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.current_screen = CurrentScreen::Exiting;
            }
            KeyCode::Char('e' | 'i') => self.current_screen = CurrentScreen::Editing,
            KeyCode::Char('j') | KeyCode::Down if key.kind == KeyEventKind::Press => {
                self.scroll_down();
            }
            KeyCode::Char('k') | KeyCode::Up if key.kind == KeyEventKind::Press => self.scroll_up(),
            KeyCode::Char('m') => self.mark_current_as_read().await?,
            KeyCode::Char('o') => self.current_screen = CurrentScreen::Opening,
            KeyCode::Char('q') => self.current_screen = CurrentScreen::Exiting,
            KeyCode::Char('?') => self.current_screen = CurrentScreen::Helping,
            KeyCode::Char('u') => self.toggle_user_sidebar(),
            _ => (),
        };
        Ok(())
    }
}
