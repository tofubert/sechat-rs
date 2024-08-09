use crate::{
    backend::nc_talk::NCTalk,
    config,
    ui::{
        chat_box::ChatBox, chat_selector::ChatSelector, help_box::HelpBox, input_box::InputBox,
        title_bar::TitleBar, users::Users,
    },
};
use ratatui::{prelude::*, widgets::Paragraph};
use std::error::Error;
use strum_macros::Display;

#[derive(PartialEq, Clone, Copy, Display)]
pub enum CurrentScreen {
    Reading,
    Opening,
    Editing,
    Exiting,
    Helping,
}

pub struct App<'a> {
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    backend: NCTalk,
    title: TitleBar<'a>,
    chat: ChatBox<'a>,
    pub selector: ChatSelector<'a>,
    input: InputBox,
    help: HelpBox,
    users: Users<'a>,
    user_sidebar_visible: bool,
}

impl<'a> App<'a> {
    pub fn new(backend: NCTalk) -> Self {
        Self {
            current_screen: CurrentScreen::Reading,
            title: TitleBar::new(
                CurrentScreen::Reading,
                backend
                    .get_room_by_token(&backend.current_room_token)
                    .to_string(),
            ),
            selector: ChatSelector::new(&backend),
            input: InputBox::default(),
            chat: {
                let mut chat = ChatBox::new();
                chat.update_messages(&backend);
                chat.select_last_message();
                chat
            },
            users: {
                let mut users = Users::new();
                users.update(&backend);
                users
            },
            backend,
            help: HelpBox::default(),
            user_sidebar_visible: config::get().data.ui.user_sidebar_default,
        }
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
                    .style(Style::default().bold().light_magenta()),
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

    pub async fn mark_current_as_read(&mut self) -> Result<(), Box<dyn Error>> {
        self.backend.get_current_room().mark_as_read().await?;
        self.backend.update_rooms(true).await?;
        self.update_ui()?;
        Ok(())
    }

    fn update_ui(&mut self) -> Result<(), Box<dyn Error>> {
        self.title.update(self.current_screen, &self.backend);
        self.selector.update(&self.backend)?;
        self.chat.update_messages(&self.backend);
        self.users.update(&self.backend);
        Ok(())
    }

    pub async fn send_message(&mut self) -> Result<(), Box<dyn Error>> {
        self.backend.send_message(self.input.to_string()).await?;
        self.input.clear();
        self.update_ui()?;
        self.chat.select_last_message();
        Ok(())
    }

    pub async fn select_room(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub async fn fetch_updates(&mut self) -> Result<(), Box<dyn Error>> {
        self.backend.update_rooms(false).await?;
        self.update_ui()?;
        Ok(())
    }

    pub fn pop_input(&mut self) {
        self.input.pop();
    }

    pub fn append_input(&mut self, new_input: char) {
        self.input.push(new_input);
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

    pub fn click_at(&mut self, position: Position) -> Result<(), Box<dyn Error>> {
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
}
