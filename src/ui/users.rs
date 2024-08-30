use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, HighlightSpacing, Row, Table, TableState},
};
use style::Styled;

use crate::backend::nc_talk::NCTalk;

pub struct Users<'a> {
    user_list: Vec<Row<'a>>,
    state: TableState,
}

impl<'a> Default for Users<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Users<'a> {
    pub fn new() -> Users<'a> {
        Users {
            user_list: vec![],
            state: TableState::default().with_offset(0).with_selected(0),
        }
    }
    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(self, area, &mut self.state.clone());
    }
    pub fn update(&mut self, backend: &NCTalk) {
        self.user_list.clear();
        let mut new_users = backend.get_current_room().get_users().clone();
        new_users.sort_by(|user1, user2| user1.displayName.cmp(&user2.displayName));
        for user in new_users {
            let mut cell = Cell::new(user.displayName.to_string());
            if let Some(status) = user.status {
                cell = match status.as_str() {
                    "away" => cell.set_style(Style::new().blue()),
                    "offline" => cell.set_style(Style::new().gray()),
                    "dnd" => cell.set_style(Style::new().red()),
                    "online" => cell.set_style(Style::new().green()),
                    unknown => {
                        log::debug!("Unknown Status {unknown}");
                        cell.set_style(Style::new())
                    }
                }
            };
            self.user_list.push(Row::new([cell]));
        }

        self.state = TableState::default().with_offset(0).with_selected(0);
    }
}

impl<'a> StatefulWidget for &Users<'a> {
    type State = TableState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidget::render(
            Table::new(self.user_list.clone(), [Constraint::Percentage(100)])
                .column_spacing(1)
                .style(Style::new().white().on_black())
                .header(Row::new(vec!["Users"]).style(Style::new().bold().green()))
                .block(Block::default())
                .highlight_style(Style::new().bold())
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
    use backend::TestBackend;

    use super::*;

    #[test]
    fn render_users() {
        let backend = TestBackend::new(10, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let users = Users::new();
        terminal
            .draw(|frame| users.render_area(frame, Rect::new(0, 0, 8, 8)))
            .unwrap();
        let mut expected = Buffer::with_lines([
            "│Users    ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "│         ",
            "          ",
            "          ",
        ]);
        expected.set_style(Rect::new(0, 0, 8, 8), Style::new().white().on_black());

        for x in 1..=7 {
            expected[(x, 0)].set_style(Style::new().green().on_black().bold());
        }

        terminal.backend().assert_buffer(&expected);
    }
}
