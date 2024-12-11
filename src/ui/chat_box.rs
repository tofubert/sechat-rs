use crate::backend::{nc_room::NCRoomInterface, nc_talk::NCBackend};
use crate::config::get_theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Cell, HighlightSpacing, Row, Table, TableState},
};
use textwrap::Options;

// this fits my name, so 20 it is :D
const NAME_WIDTH: u16 = 20;
const TIME_WIDTH: u16 = 5;

#[derive(Default)]
pub struct ChatBox<'a> {
    messages: Vec<Row<'a>>,
    current_index: usize,
    width: u16,
    state: TableState,
}

impl ChatBox<'_> {
    pub fn new() -> Self {
        ChatBox {
            messages: Vec::new(),
            current_index: 0,
            width: 10,
            state: TableState::default().with_offset(1).with_selected(0),
        }
    }

    pub fn set_width_and_update_if_change(&mut self, width: u16, backend: &impl NCBackend) {
        let new_width = (width - TIME_WIDTH - 2 - NAME_WIDTH).max(10);
        if self.width != new_width {
            self.width = new_width;
            self.update_messages(backend);
        }
    }

    pub fn update_messages(&mut self, backend: &impl NCBackend) {
        use itertools::Itertools;
        use std::convert::TryInto;

        self.messages.clear();
        for message_data in backend
            .get_current_room()
            .get_messages()
            .iter()
            .filter(|mes| !mes.is_reaction() && !mes.is_edit_note() && !mes.is_comment_deleted())
        {
            let name = textwrap::wrap(
                message_data.get_name(),
                Options::new(NAME_WIDTH.into()).break_words(true),
            )
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .map(Line::from)
            .collect_vec();

            let message_string = message_data
                .get_message()
                .split('\n')
                .flat_map(|cell| {
                    textwrap::wrap(cell, self.width as usize)
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .map(Line::from)
                        .collect_vec()
                })
                .collect_vec();

            let row_height: u16 = if message_string.len() > name.len() {
                message_string.len().try_into().expect("message too long")
            } else {
                name.len().try_into().expect("name too long")
            };
            let message: Vec<Cell> = vec![
                message_data.get_time_str().into(),
                name.into(),
                message_string.into(),
            ];

            self.messages.push(Row::new(message).height(row_height));

            if message_data.has_reactions() {
                let reaction: Vec<Cell> = vec![
                    "".into(),
                    "".into(),
                    message_data.get_reactions_str().into(),
                ];
                self.messages.push(Row::new(reaction));
            }
            if backend.get_current_room().has_unread()
                && backend.get_current_room().get_last_read() == message_data.get_id()
            {
                let unread_marker: Vec<Cell> = vec![
                    "".into(),
                    "".into(),
                    Span::styled(
                        "+++ LAST READ +++",
                        get_theme()
                            .unread_message_style()
                            .add_modifier(Modifier::BOLD),
                    )
                    .into(),
                ];
                self.messages.push(Row::new(unread_marker));
            }
        }
    }

    pub fn select_last_message(&mut self) {
        self.current_index = self.messages.len().saturating_sub(1);
        self.state.select(Some(self.current_index));
    }

    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(self, area, &mut self.state.clone());
    }

    pub fn select_up(&mut self) {
        self.current_index = self
            .current_index
            .saturating_sub(1)
            .clamp(0, self.messages.len() - 1);
        self.state.select(Some(self.current_index));
    }

    pub fn select_down(&mut self) {
        self.current_index = self
            .current_index
            .saturating_add(1)
            .clamp(0, self.messages.len() - 1);
        self.state.select(Some(self.current_index));
    }
    pub fn select_line(&mut self, position: Position) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "Got Position {:?} and selected {:?}",
            position,
            self.state.selected().ok_or("nothing selected")?
        );

        // let new_selection = state.selected().ok_or("nothing selected")?;
        // self.current_index = position
        //     .y
        //     .clamp(0, (self.messages.len() - 1).try_into()?)
        //     .try_into()?;
        // Ok(())
        todo!("commented code missing?");
    }
}

impl<'a> StatefulWidget for &ChatBox<'a> {
    type State = TableState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Columns widths are constrained in the same way as Layout...
        let widths = [
            Constraint::Length(TIME_WIDTH),
            Constraint::Length(NAME_WIDTH),
            Constraint::Min(10),
        ];
        StatefulWidget::render(
            Table::new(self.messages.clone(), widths)
                .column_spacing(1)
                .style(get_theme().default_style())
                .header(
                    Row::new(vec!["Time", "Name", "Message"])
                        .style(get_theme().table_header_style()),
                )
                .block(Block::default())
                .highlight_style(get_theme().default_highlight_style())
                .highlight_spacing(HighlightSpacing::Never),
            area,
            buf,
            state,
        );
    }
}
