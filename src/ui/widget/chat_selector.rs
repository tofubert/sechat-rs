use std::error::Error;

use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{Block, Scrollbar, ScrollbarOrientation},
    Frame,
};

use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::backend::nc_room::NCRoomInterface;
use crate::backend::nc_talk::NCBackend;
use crate::config::Config;

pub struct ChatSelector<'a> {
    pub state: TreeState<String>,
    items: Vec<TreeItem<'a, String>>,
    default_style: Style,
    default_highlight_style: Style,
}

impl ChatSelector<'_> {
    pub fn new(backend: &impl NCBackend, config: &Config) -> Self {
        Self {
            state: TreeState::default(),
            items: vec![
                TreeItem::new::<String>(
                    "unread".to_string(),
                    "Unread Chats".to_string(),
                    backend
                        .get_unread_rooms()
                        .iter()
                        .map(|token| {
                            TreeItem::new_leaf::<String>(
                                token.to_string(),
                                backend.get_room(token).get_display_name().into(),
                            )
                        })
                        .collect_vec(),
                )
                .expect("unread duplicate"),
                TreeItem::new::<String>(
                    "favorites".to_string(),
                    "Favorite Chats".to_string(),
                    backend
                        .get_favorite_rooms()
                        .iter()
                        .map(|token| {
                            TreeItem::new_leaf::<String>(
                                token.to_string(),
                                backend.get_room(token).get_display_name().into(),
                            )
                        })
                        .collect_vec(),
                )
                .expect("favorite room name duplicate"),
                TreeItem::new::<String>(
                    "direct".to_string(),
                    "DMs".to_string(),
                    backend
                        .get_dm_keys_display_name_mapping()
                        .iter()
                        .map(|(token, display_name)| {
                            TreeItem::new_leaf::<String>(token.to_string(), display_name.clone())
                        })
                        .collect_vec(),
                )
                .expect("DM name duplicate"),
                TreeItem::new::<String>(
                    "group".to_string(),
                    "Group".to_string(),
                    backend
                        .get_group_keys_display_name_mapping()
                        .iter()
                        .map(|(token, display_name)| {
                            TreeItem::new_leaf::<String>(token.to_string(), display_name.clone())
                        })
                        .collect_vec(),
                )
                .expect("Group name duplicate"),
            ],
            default_style: config.theme.default_style(),
            default_highlight_style: config.theme.default_highlight_style(),
        }
    }

    pub fn update(&mut self, backend: &impl NCBackend) -> Result<(), Box<dyn Error>> {
        self.items = vec![
            TreeItem::new::<String>(
                "unread".to_string(),
                "Unread Chats".to_string(),
                backend
                    .get_unread_rooms()
                    .iter()
                    .map(|token| {
                        TreeItem::new_leaf::<String>(
                            token.to_string(),
                            backend.get_room(token).get_display_name().into(),
                        )
                    })
                    .collect_vec(),
            )?,
            TreeItem::new::<String>(
                "favorites".to_string(),
                "Favorite Chats".to_string(),
                backend
                    .get_favorite_rooms()
                    .iter()
                    .map(|token| {
                        TreeItem::new_leaf::<String>(
                            token.to_string(),
                            backend.get_room(token).get_display_name().into(),
                        )
                    })
                    .collect_vec(),
            )?,
            TreeItem::new::<String>(
                "direct".to_string(),
                "DMs".to_string(),
                backend
                    .get_dm_keys_display_name_mapping()
                    .iter()
                    .map(|(token, display_name)| {
                        TreeItem::new_leaf::<String>(token.to_string(), display_name.clone())
                    })
                    .collect_vec(),
            )?,
            TreeItem::new::<String>(
                "group".to_string(),
                "Group".to_string(),
                backend
                    .get_group_keys_display_name_mapping()
                    .iter()
                    .map(|(token, display_name)| {
                        TreeItem::new_leaf::<String>(token.to_string(), display_name.clone())
                    })
                    .collect_vec(),
            )?,
        ];
        Ok(())
    }

    pub fn render_area(&mut self, frame: &mut Frame, area: Rect) {
        let widget = Tree::new(&self.items)
            .expect("all item identifiers are unique")
            .block(Block::bordered().title("Chat Section"))
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .style(self.default_style)
            .highlight_style(self.default_highlight_style.bold())
            .highlight_symbol(">> ");
        frame.render_stateful_widget(widget, area, &mut self.state);
    }
}

#[cfg(test)]
mod tests {

    use crate::backend::nc_request::{NCReqDataParticipants, Token};
    use crate::backend::nc_room::MockNCRoomInterface;
    use crate::backend::nc_talk::MockNCTalk;
    use crate::config::init;
    use backend::TestBackend;
    use mockall::predicate::eq;
    use mockall::Sequence;

    use super::*;

    #[test]
    fn render() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        let mut seq = Sequence::new();

        let mut mock_nc_backend = MockNCTalk::new();
        let mut mock_room = MockNCRoomInterface::new();
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        mock_nc_backend
            .expect_get_unread_rooms()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![]);

        mock_nc_backend
            .expect_get_favorite_rooms()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![]);

        mock_nc_backend
            .expect_get_dm_keys_display_name_mapping()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![]);

        mock_nc_backend
            .expect_get_group_keys_display_name_mapping()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![]);

        mock_nc_backend
            .expect_get_unread_rooms()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![Token::from("0")]);

        mock_room
            .expect_get_display_name()
            .once()
            .return_const("General".to_string());

        mock_nc_backend
            .expect_get_room()
            .with(eq(Token::from("0")))
            .once()
            .in_sequence(&mut seq)
            .return_const(mock_room);

        mock_nc_backend
            .expect_get_favorite_rooms()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![]);

        mock_nc_backend
            .expect_get_dm_keys_display_name_mapping()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![(Token::from("Butz"), "1".to_string())]);

        mock_nc_backend
            .expect_get_group_keys_display_name_mapping()
            .once()
            .in_sequence(&mut seq)
            .return_const(vec![(Token::from("Bert"), "2".to_string())]);

        let mut chat_selector_box = ChatSelector::new(&mock_nc_backend, &config);

        let mut dummy_user = NCReqDataParticipants::default();
        dummy_user.displayName = "Butz".to_string();

        terminal
            .draw(|frame| chat_selector_box.render_area(frame, Rect::new(0, 0, 40, 10)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "┌Chat Section──────────────────────────┐",
            "│  Unread Chats                        │",
            "│  Favorite Chats                      │",
            "│  DMs                                 │",
            "│  Group                               │",
            "│                                      │",
            "│                                      │",
            "│                                      │",
            "│                                      │",
            "└──────────────────────────────────────┘",
        ]);
        expected.set_style(Rect::new(0, 0, 40, 10), config.theme.default_style());

        terminal.backend().assert_buffer(&expected);

        assert!(chat_selector_box.update(&mock_nc_backend).is_ok());

        chat_selector_box.state.key_down();
        chat_selector_box.state.key_right();

        terminal
            .draw(|frame| chat_selector_box.render_area(frame, Rect::new(0, 0, 40, 10)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "┌Chat Section──────────────────────────┐",
            "│>> ▼ Unread Chats                     │",
            "│       General                        │",
            "│     Favorite Chats                   │",
            "│   ▶ DMs                              │",
            "│   ▶ Group                            │",
            "│                                      │",
            "│                                      │",
            "│                                      │",
            "└──────────────────────────────────────┘",
        ]);
        expected.set_style(Rect::new(0, 0, 40, 10), config.theme.default_style());
        expected.set_style(
            Rect::new(1, 1, 38, 1),
            config.theme.default_highlight_style().bold(),
        );

        terminal.backend().assert_buffer(&expected);
    }
}
