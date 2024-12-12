use crate::backend::nc_room::NCRoomInterface;
use crate::config::Config;
use crate::{backend::nc_talk::NCBackend, ui::app::CurrentScreen};

use num_traits::AsPrimitive as _;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use style::Styled;

pub struct TitleBar<'a> {
    room: String,
    mode: String,
    unread: usize,
    unread_rooms: Text<'a>,
    title_important_style: Style,
    title_style: Style,
    default_style: Style,
}

impl TitleBar<'_> {
    pub fn new(initial_state: CurrentScreen, room: String, config: &Config) -> Self {
        TitleBar {
            room,
            mode: initial_state.to_string(),
            unread: 0,
            unread_rooms: Text::raw(""),
            title_important_style: config.theme.title_important_style().rapid_blink(),
            title_style: config.theme.title_status_style(),
            default_style: config.theme.default_style(),
        }
    }

    pub fn update(&mut self, screen: CurrentScreen, backend: &impl NCBackend) {
        self.mode = screen.to_string();
        self.room = backend.get_current_room().to_string();
        self.unread = backend.get_current_room().get_unread();
        let unread_array: Vec<String> = backend
            .get_unread_rooms()
            .iter()
            .map(|token| {
                let room = backend.get_room(token);
                format!("{room}: {}", room.get_unread())
            })
            .collect();
        self.unread_rooms = if unread_array.is_empty() {
            Text::raw("")
        } else {
            Text::raw("UNREAD: ".to_owned() + unread_array.join(", ").as_str())
                .set_style(self.title_important_style)
        };
    }

    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self, area);
    }
}

impl Widget for &TitleBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (room_title, room_title_style) = if self.unread > 0 {
            (
                format!("Current: {}: {}", self.room, self.unread),
                self.title_style,
            )
        } else {
            (format!("Current: {}", self.room), self.title_style)
        };

        let title_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(room_title.len().as_()),
                Constraint::Fill(1),
                Constraint::Percentage(20),
            ])
            .split(area);

        let title_block = Block::default()
            .borders(Borders::BOTTOM)
            .style(self.default_style);

        Paragraph::new(Text::styled(room_title, room_title_style))
            .block(title_block)
            .render(title_layout[0], buf);

        let unread_block = Block::default()
            .borders(Borders::BOTTOM)
            .style(self.default_style);

        Paragraph::new(self.unread_rooms.clone())
            .block(unread_block)
            .render(title_layout[1], buf);

        let mode_block = Block::default()
            .borders(Borders::BOTTOM)
            .style(self.default_style);

        Paragraph::new(Text::styled(self.mode.clone(), self.title_style))
            .block(mode_block)
            .alignment(Alignment::Right)
            .render(title_layout[2], buf);
    }
}
