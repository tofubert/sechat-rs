use itertools::Itertools;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, HighlightSpacing, Row, Table, TableState},
};
use style::Styled;

use crate::backend::{nc_room::NCRoomInterface, nc_talk::NCBackend};
use crate::config::Config;

pub struct Users<'a> {
    user_list: Vec<Row<'a>>,
    state: TableState,
    default_style: Style,
    user_away_style: Style,
    user_dnd_style: Style,
    user_online_style: Style,
    user_offline_style: Style,
    table_header_style: Style,
}

impl Users<'_> {
    pub fn new(config: &Config) -> Self {
        Users {
            user_list: vec![],
            state: TableState::default().with_offset(0).with_selected(0),
            default_style: config.theme.default_style(),
            user_away_style: config.theme.user_away_style(),
            user_dnd_style: config.theme.user_dnd_style(),
            user_online_style: config.theme.user_online_style(),
            user_offline_style: config.theme.user_offline_style(),
            table_header_style: config.theme.table_header_style(),
        }
    }
    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(self, area, &mut self.state.clone());
    }
    pub fn update(&mut self, backend: &impl NCBackend) {
        self.user_list = backend
            .get_current_room()
            .get_users()
            .iter()
            .sorted_by(|user1, user2| user1.displayName.cmp(&user2.displayName))
            .map(|user| {
                Row::new([{
                    if let Some(status) = &user.status {
                        Cell::new(user.displayName.to_string()).set_style(match status.as_str() {
                            "away" => self.user_away_style,
                            "offline" => self.user_offline_style,
                            "dnd" => self.user_dnd_style,
                            "online" => self.user_online_style,
                            unknown => {
                                log::debug!("Unknown Status {unknown}");
                                self.default_style
                            }
                        })
                    } else {
                        Cell::new(user.displayName.to_string()).style(self.default_style)
                    }
                }])
            })
            .collect();

        self.state = TableState::default().with_offset(0).with_selected(0);
    }
}

impl StatefulWidget for &Users<'_> {
    type State = TableState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidget::render(
            Table::new(self.user_list.clone(), [Constraint::Percentage(100)])
                .column_spacing(1)
                .style(self.default_style)
                .header(Row::new(vec!["Users"]).style(self.table_header_style))
                .block(Block::default())
                .row_highlight_style(Style::new().bold())
                .highlight_spacing(HighlightSpacing::Never)
                .highlight_symbol("")
                .block(Block::new().borders(Borders::LEFT)),
            area,
            buf,
            state,
        );
    }
}

#[cfg(test)]
mod tests {

    use crate::backend::{
        nc_request::NCReqDataParticipants, nc_room::MockNCRoomInterface, nc_talk::MockNCTalk,
    };
    use crate::config::init;
    use backend::TestBackend;

    use super::*;

    #[test]
    fn render_users() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();

        let mut mock_nc_backend = MockNCTalk::new();
        let backend = TestBackend::new(10, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut users = Users::new(&config);

        let mut mock_room = MockNCRoomInterface::new();
        let mut dummy_user = NCReqDataParticipants::default();
        dummy_user.displayName = "Butz".to_string();
        mock_room.expect_get_users().return_const(vec![dummy_user]);
        mock_nc_backend
            .expect_get_current_room()
            .once()
            .return_const(mock_room);
        users.update(&mock_nc_backend);

        terminal
            .draw(|frame| users.render_area(frame, Rect::new(0, 0, 8, 8)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "│Users    ",
            "│Butz     ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "          ",
            "          ",
        ]);
        expected.set_style(Rect::new(0, 0, 8, 8), config.theme.default_style());

        // header
        for x in 1..=7 {
            expected[(x, 0)].set_style(config.theme.table_header_style());
        }

        // selected user
        for x in 1..=7 {
            expected[(x, 1)].set_style(config.theme.default_style().bold());
        }

        terminal.backend().assert_buffer(&expected);
    }
}
