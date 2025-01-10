//! App holds the main event loop, creates and tears down the ui.
//!
//! ### General
//! The structure of the Frontend takes a lot of inspiration from [crates-tui](https://github.com/ratatui/crates-tui/tree/main).
//! The [ratatui Widgets](https://docs.rs/ratatui/latest/ratatui/widgets/trait.Widget.html) in the
//! [`widget`](crate::ui::widget) module use ratatui widgets as well as the following extra crates:
//! * [tui-tree-widget](https://github.com/EdJoPaTo/tui-rs-tree-widget)
//! * [tui-textarea](https://github.com/rhysd/tui-textarea)
//!
//! ### Structure
//! The [`App`] holds all top level state and all objects.
//! It stores the [``NCTalk``](crate::backend::nc_talk::NCTalk) instance, the [``notification object``](crate::ui::notifications::NotifyWrapper)
//! and the [``current screen``](crate::ui::app::CurrentScreen).
//!
//! The [``run``](crate::ui::app::App::run) method does the ui setup, through the [``init``] function,
//! and then calls [``run_ui``](crate::ui::app::App::run_app) to execute the main loop.
//! the main loop ether waits for a key event. Should now event ocure for 3 seconds a update from the remote server is fetched.
use crate::{
    backend::{nc_request::Token, nc_room::NCRoomInterface, nc_talk::NCBackend},
    config::Config,
    ui::terminal_helpers::{init, install_hooks, restore},
    ui::widget::{
        chat_box::ChatBox, chat_selector::ChatSelector, help_box::HelpBox, input_box::InputBox,
        title_bar::TitleBar, users::Users,
    },
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout, Position},
    style::{Style, Stylize},
    widgets::{Block, Clear, Paragraph},
    Frame, Terminal,
};
use strum_macros::Display;

use tui_textarea::Input;

use crossterm::event::{
    poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use tui_textarea::Key;

use super::notifications::NotifyWrapper;

enum ProcessEventResult {
    Continue,
    Exit,
}

#[derive(PartialEq, Clone, Copy, Display)]
pub enum CurrentScreen {
    Reading,
    Opening,
    Editing,
}

#[derive(PartialEq, Clone, Copy, Display)]
pub enum Popup {
    Help,
    Exit,
}

pub struct App<'a, Backend: NCBackend> {
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    popup: Option<Popup>,
    backend: Backend,
    title: TitleBar<'a>,
    chat: ChatBox<'a>,
    pub selector: ChatSelector<'a>,
    input: InputBox<'a>,
    help: HelpBox,
    users: Users<'a>,
    user_sidebar_visible: bool,
    default_style: Style,
    popup_border_style: Style,
    current_room_token: Token,
    notify: NotifyWrapper,
}

impl<Backend: NCBackend> App<'_, Backend> {
    pub fn new(backend: Backend, config: &Config) -> Self {
        let init_room = backend.get_room_by_displayname(config.data.ui.default_room.as_str());
        let notify = NotifyWrapper::new(config);

        Self {
            current_screen: CurrentScreen::Reading,
            popup: None,
            title: TitleBar::new(CurrentScreen::Reading, config),
            selector: ChatSelector::new(&backend, config),
            input: InputBox::new("", config),
            chat: {
                let mut chat = ChatBox::new(config);
                chat.update_messages(&backend, &init_room);
                chat.select_last_message();
                chat
            },
            users: {
                let mut users = Users::new(config);
                users.update(&backend, &init_room);
                users
            },
            backend,
            help: HelpBox::new(config),
            user_sidebar_visible: config.data.ui.user_sidebar_default,
            default_style: config.theme.default_style(),
            popup_border_style: config.theme.popup_border_style(),
            current_room_token: init_room,
            notify,
        }
    }

    pub async fn run(&mut self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        install_hooks(config)?;

        let tui = init(config.get_enable_mouse(), config.get_enable_paste())
            .expect("Could not Create TUI Backend.");

        // create app and run it
        self.run_app(tui).await?;

        // Kill worker threads.
        self.backend.shutdown().await?;

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
        } else {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(4), Constraint::Length(3)])
                .split(base_layout[1]);

            if self.user_sidebar_visible
                && self.backend.get_room(&self.current_room_token).is_group()
            {
                let chat_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                    .split(main_layout[0]);
                self.chat.set_width_and_update_if_change(
                    chat_layout[0].width,
                    &self.backend,
                    &self.current_room_token,
                );
                self.chat.render_area(f, chat_layout[0]);
                self.users.render_area(f, chat_layout[1]);
            } else {
                self.chat.set_width_and_update_if_change(
                    main_layout[0].width,
                    &self.backend,
                    &self.current_room_token,
                );
                self.chat.render_area(f, main_layout[0]);
            };

            self.input.render_area(f, main_layout[1]);
        }
        self.title
            .update(self.current_screen, &self.backend, &self.current_room_token);
        self.title.render_area(f, base_layout[0]);
        if let Some(popup) = self.popup {
            let (horizontal, vertical) = match popup {
                Popup::Help => (Constraint::Length(130), Constraint::Length(12)),
                Popup::Exit => (Constraint::Length(40), Constraint::Length(3)),
            };
            let [area] = Layout::horizontal([horizontal])
                .flex(Flex::Center)
                .areas(base_layout[1]);
            let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
            f.render_widget(Clear, area);
            match popup {
                Popup::Help => self.help.render_area(f, area),
                Popup::Exit => f.render_widget(
                    Paragraph::new("To Quit Press 'y', to stay 'n'")
                        .alignment(Alignment::Center)
                        .style(self.default_style.bold())
                        .block(
                            Block::bordered()
                                .title("Exit?")
                                .border_style(self.popup_border_style),
                        ),
                    area,
                ),
            }
        }
    }

    pub async fn mark_current_as_read(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.backend
            .mark_current_room_as_read(&self.current_room_token)
            .await?;
        self.notify
            .maybe_notify_new_rooms(self.backend.update_rooms(true).await?)?;
        self.update_ui()?;
        Ok(())
    }

    fn update_ui(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.title
            .update(self.current_screen, &self.backend, &self.current_room_token);
        self.selector.update(&self.backend)?;
        self.chat
            .update_messages(&self.backend, &self.current_room_token);
        self.users.update(&self.backend, &self.current_room_token);
        Ok(())
    }

    pub async fn send_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.input.is_empty() {
            Ok(())
        } else {
            self.notify.maybe_notify_new_message(
                self.backend
                    .send_message(self.input.lines().join("\n"), &self.current_room_token)
                    .await?,
            )?;
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
            self.current_room_token.clone_from(
                self.selector
                    .state
                    .selected()
                    .last()
                    .expect("no selection available"),
            );
            self.notify.maybe_notify_new_message(
                self.backend.select_room(&self.current_room_token).await?,
            )?;
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
            CurrentScreen::Editing => (),
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
                if let Some(popup) = self.popup {
                    match popup {
                        Popup::Help => self.handle_key_in_help(key),
                        Popup::Exit => {
                            if let Some(value) = self.handle_key_in_exit(key) {
                                return value;
                            }
                        }
                    }
                }
                match self.current_screen {
                    CurrentScreen::Reading => self.handle_key_in_reading(key).await?,
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
            KeyCode::Char('d') | KeyCode::PageDown => {
                _ = self.selector.state.select_relative(|current| {
                    current.map_or(0, |current| current.saturating_add(9))
                });
            }
            KeyCode::Char('u') | KeyCode::PageUp => {
                _ = self.selector.state.select_relative(|current| {
                    current.map_or(0, |current| current.saturating_sub(9))
                });
            }
            KeyCode::Char('q') => self.popup = Some(Popup::Exit),
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Char(' ') => _ = self.selector.state.toggle_selected(),
            KeyCode::Enter => self.select_room().await?,
            KeyCode::Home => _ = self.selector.state.select_first(),
            KeyCode::End => _ = self.selector.state.select_last(),
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
                self.mark_current_as_read().await?;
                self.send_message().await?;
            }
            _ => self.new_input_key(key),
        };

        Ok(())
    }

    fn handle_key_in_help(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.popup = Some(Popup::Exit),
            KeyCode::Esc => self.popup = None,
            KeyCode::Char('o') => {
                self.popup = None;
                self.current_screen = CurrentScreen::Opening;
            }
            _ => (),
        }
    }

    fn handle_key_in_exit(
        &mut self,
        key: KeyEvent,
    ) -> Option<Result<ProcessEventResult, Box<dyn std::error::Error>>> {
        match key.code {
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Char('y') => {
                if let Err(err) = self.write_log_files() {
                    log::warn!(
                        "Failure to store logs into log file ({}), ignoring for now.",
                        err
                    );
                }
                return Some(Ok(ProcessEventResult::Exit));
            }
            KeyCode::Char('n') => self.popup = None,
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
                self.popup = Some(Popup::Exit);
            }
            KeyCode::Char('e' | 'i') => self.current_screen = CurrentScreen::Editing,
            KeyCode::Char('j') | KeyCode::Down if key.kind == KeyEventKind::Press => {
                self.scroll_down();
            }
            KeyCode::Char('k') | KeyCode::Up if key.kind == KeyEventKind::Press => self.scroll_up(),
            KeyCode::Char('m') => self.mark_current_as_read().await?,
            KeyCode::Char('o') => self.current_screen = CurrentScreen::Opening,
            KeyCode::Char('q') => self.popup = Some(Popup::Exit),
            KeyCode::Char('?') => self.popup = Some(Popup::Help),
            KeyCode::Char('u') => self.toggle_user_sidebar(),
            _ => (),
        };
        Ok(())
    }
}
