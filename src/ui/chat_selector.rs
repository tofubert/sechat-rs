use std::error::Error;

use itertools::Itertools;
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Scrollbar, ScrollbarOrientation},
    Frame,
};

use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::backend::nc_talk::NCBackend;

pub struct ChatSelector<'a> {
    pub state: TreeState<String>,
    items: Vec<TreeItem<'a, String>>,
}

impl<'a> ChatSelector<'a> {
    pub fn new(backend: &dyn NCBackend) -> Self {
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
                                token.clone(),
                                backend.get_room_by_token(token).get_display_name().into(),
                            )
                        })
                        .collect_vec(),
                )
                .expect("unread duplicate"),
                TreeItem::new::<String>(
                    "direct".to_string(),
                    "DMs".to_string(),
                    backend
                        .get_dm_keys_display_name_mapping()
                        .iter()
                        .map(|(token, display_name)| {
                            TreeItem::new_leaf::<String>(token.clone(), display_name.clone())
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
                            TreeItem::new_leaf::<String>(token.clone(), display_name.clone())
                        })
                        .collect_vec(),
                )
                .expect("Group name duplicate"),
            ],
        }
    }

    pub fn update(&mut self, backend: &dyn NCBackend) -> Result<(), Box<dyn Error>> {
        self.items = vec![
            TreeItem::new::<String>(
                "unread".to_string(),
                "Unread Chats".to_string(),
                backend
                    .get_unread_rooms()
                    .iter()
                    .map(|token| {
                        TreeItem::new_leaf::<String>(
                            token.clone(),
                            backend.get_room_by_token(token).get_display_name().into(),
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
                        TreeItem::new_leaf::<String>(token.clone(), display_name.clone())
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
                        TreeItem::new_leaf::<String>(token.clone(), display_name.clone())
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
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(widget, area, &mut self.state);
    }
}
