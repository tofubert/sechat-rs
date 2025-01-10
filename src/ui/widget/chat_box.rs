use crate::backend::nc_message::NCMessage;
use crate::backend::nc_request::Token;
use crate::backend::{nc_room::NCRoomInterface, nc_talk::NCBackend};
use crate::config::Config;
use chrono::{DateTime, Local, Utc};
use itertools::Itertools;
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
    default_style: Style,
    default_highlight_style: Style,
    unread_message_style: Style,
    table_header_style: Style,
    date_format: String,
}

impl ChatBox<'_> {
    pub fn new(config: &Config) -> Self {
        ChatBox {
            messages: Vec::new(),
            current_index: 0,
            width: 10,
            state: TableState::default().with_offset(0).with_selected(0),
            unread_message_style: config
                .theme
                .unread_message_style()
                .add_modifier(Modifier::BOLD),
            default_style: config.theme.default_style(),
            default_highlight_style: config.theme.default_highlight_style(),
            table_header_style: config.theme.table_header_style(),
            date_format: config.data.ui.date_format.clone(),
        }
    }

    pub fn set_width_and_update_if_change(
        &mut self,
        width: u16,
        backend: &impl NCBackend,
        current_room: &Token,
    ) {
        let new_width = (width - TIME_WIDTH - 2 - NAME_WIDTH).max(10);
        if self.width != new_width {
            self.width = new_width;
            self.update_messages(backend, current_room);
        }
    }

    pub fn update_messages(&mut self, backend: &impl NCBackend, current_room: &Token) {
        use itertools::Itertools;
        use std::convert::TryInto;

        // Remove all previous messages.
        self.messages.clear();

        let mut last_date = DateTime::<Utc>::MIN_UTC
            .format(&self.date_format)
            .to_string();

        // iterate over all messages.
        for message_data in backend
            .get_room(current_room)
            .get_messages()
            .values()
            .filter(|mes| !mes.is_reaction() && !mes.is_edit_note() && !mes.is_comment_deleted())
        {
            // Create the Date Section.
            let date_str = message_data.get_date_str(&self.date_format);
            if date_str != last_date {
                let mut date: Vec<Cell> = vec![
                    "".into(),
                    "".into(),
                    Span::styled(date_str.clone(), self.unread_message_style).into(),
                ];
                if date_str == Local::now().format(&self.date_format).to_string() {
                    let today_str = String::from("Today! ");
                    date = vec![
                        "".into(),
                        "".into(),
                        Span::styled(today_str + date_str.as_str(), self.unread_message_style)
                            .into(),
                    ];
                }
                self.messages.push(Row::new(date));
                last_date = date_str;
            }

            // Create the name Section.
            let name = textwrap::wrap(
                message_data.get_name().to_string().as_str(),
                Options::new(NAME_WIDTH.into()).break_words(true),
            )
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .map(Line::from)
            .collect_vec();

            // Format the message
            let message_string = self.format_message(message_data);

            // figure out how high this Row needs to be.
            let row_height: u16 = if message_string.len() > name.len() {
                message_string.len().try_into().expect("message too long")
            } else {
                name.len().try_into().expect("name too long")
            };
            // Put all 3 parts into Line Vector.
            let message: Vec<Cell> = vec![
                message_data.get_time_str().into(),
                name.into(),
                message_string.into(),
            ];

            // Add Message to Messages Vector
            self.messages.push(Row::new(message).height(row_height));

            // If Message has Reactions we add those as the next line.
            self.insert_reaction_if_needed(message_data);

            self.insert_unread_marker_if_needed(backend, current_room, message_data);
        }
    }

    pub fn select_last_message(&mut self) {
        log::trace!("messages length: {}", self.messages.len());
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

    /// check if the Room has unread messages and if so insert the Unread Marker.
    fn insert_unread_marker_if_needed(
        &mut self,
        backend: &impl NCBackend,
        current_room: &Token,
        message_data: &NCMessage,
    ) {
        if backend.get_room(current_room).has_unread()
            && backend.get_room(current_room).get_last_read() == message_data.get_id()
        {
            let unread_marker: Vec<Cell> = vec![
                "".into(),
                "".into(),
                Span::styled("+++ LAST READ +++", self.unread_message_style).into(),
            ];
            self.messages.push(Row::new(unread_marker));
        }
    }

    /// Push a line with the reactions of the Message into the message vector.
    fn insert_reaction_if_needed(&mut self, message_data: &NCMessage) {
        if message_data.has_reactions() {
            let reaction: Vec<Cell> = vec![
                "".into(),
                "".into(),
                message_data.get_reactions_str().into(),
            ];
            self.messages.push(Row::new(reaction));
        }
    }

    fn format_message<'a>(&mut self, message_data: &NCMessage) -> Vec<Line<'a>> {
        let mut message_text = message_data.get_message().to_string();
        if let Some(params) = message_data.get_message_params() {
            for (key, value) in params {
                message_text = message_text.replace(key, &value.name);
            }
        }
        message_text
            .split('\n')
            .flat_map(|cell| {
                textwrap::wrap(cell, self.width as usize)
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .map(Line::from)
                    .collect_vec()
            })
            .collect_vec()
    }
}

impl StatefulWidget for &ChatBox<'_> {
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
                .style(self.default_style)
                .header(Row::new(vec!["Time", "Name", "Message"]).style(self.table_header_style))
                .block(Block::default())
                .row_highlight_style(self.default_highlight_style)
                .highlight_spacing(HighlightSpacing::Never),
            area,
            buf,
            state,
        );
    }
}

#[cfg(test)]
mod tests {

    use std::collections::BTreeMap;

    use crate::backend::nc_message::NCMessage;
    use crate::backend::nc_request::{
        NCReqDataMessage, NCReqDataMessageType, NCReqDataParticipants,
    };
    use crate::backend::nc_room::MockNCRoomInterface;
    use crate::backend::nc_talk::MockNCTalk;
    use crate::config::init;
    use backend::TestBackend;
    use chrono::{DateTime, Local, Utc};

    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn render() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();

        let mut mock_nc_backend = MockNCTalk::new();
        let mut mock_room = MockNCRoomInterface::new();
        let timestamp_1 = DateTime::<Utc>::from_timestamp(2000, 0).unwrap();
        let mock_message_1 = NCMessage::from(NCReqDataMessage {
            id: 0,
            message: "Butz".to_string(),
            messageType: NCReqDataMessageType::Comment,
            actorDisplayName: "Hundi".to_string(),
            timestamp: timestamp_1.timestamp(),
            ..Default::default()
        });
        let timestamp_2 = DateTime::<Utc>::from_timestamp(200_000, 0).unwrap();
        let mock_message_2 = NCMessage::from(NCReqDataMessage {
            id: 1,
            message: "Bert".to_string(),
            messageType: NCReqDataMessageType::Comment,
            actorDisplayName: "Stinko".to_string(),
            timestamp: timestamp_2.timestamp(),
            ..Default::default()
        });
        let message_tree = BTreeMap::from([(1, mock_message_1), (2, mock_message_2)]);

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut chat_box = ChatBox::new(&config);

        let mut dummy_user = NCReqDataParticipants::default();
        dummy_user.displayName = "Butz".to_string();
        mock_room
            .expect_get_messages()
            .once()
            .return_const(message_tree);
        mock_room.expect_has_unread().times(2).return_const(false);
        mock_nc_backend
            .expect_get_room()
            .times(3)
            .return_const(mock_room);

        terminal
            .draw(|frame| chat_box.render_area(frame, Rect::new(0, 0, 40, 10)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "Time  Name                 Message      ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
        ]);
        expected.set_style(Rect::new(0, 0, 40, 10), config.theme.default_style());

        expected.set_style(Rect::new(0, 0, 40, 1), config.theme.table_header_style());

        terminal.backend().assert_buffer(&expected);

        chat_box.update_messages(&mock_nc_backend, &"123".to_string());

        terminal
            .draw(|frame| chat_box.render_area(frame, Rect::new(0, 0, 40, 10)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "Time  Name                 Message      ",
            "                           Thursday 01 J",
            "01:33 Hundi                Butz         ",
            "                           Saturday 03 J",
            "08:33 Stinko               Bert         ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
        ]);
        expected.set_style(Rect::new(0, 0, 40, 10), config.theme.default_style());
        expected.set_style(Rect::new(0, 0, 40, 1), config.theme.table_header_style());
        expected.set_style(
            Rect::new(0, 1, 40, 1),
            config.theme.default_highlight_style(),
        );
        expected.set_style(
            Rect::new(27, 1, 13, 1),
            config
                .theme
                .default_highlight_style()
                .add_modifier(Modifier::BOLD),
        );
        expected.set_style(
            Rect::new(27, 3, 13, 1),
            config
                .theme
                .unread_message_style()
                .add_modifier(Modifier::BOLD),
        );
        expected.set_string(
            0,
            2,
            DateTime::<Local>::from(timestamp_1)
                .format("%H:%M")
                .to_string(),
            config.theme.default_style(),
        );
        expected.set_string(
            0,
            4,
            DateTime::<Local>::from(timestamp_2)
                .format("%H:%M")
                .to_string(),
            config.theme.default_style(),
        );

        terminal.backend().assert_buffer(&expected);
    }
}
