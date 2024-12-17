use crate::config::Config;
use ratatui::{
    prelude::*,
    widgets::{Block, HighlightSpacing, Row, Table},
};

#[derive(Default)]
pub struct HelpBox {
    default: Style,
    default_highlight: Style,
    table_header: Style,
}

impl HelpBox {
    pub fn new(config: &Config) -> Self {
        HelpBox {
            default: config.theme.default_style(),
            default_highlight: config.theme.default_highlight_style(),
            table_header: config.theme.table_header_style(),
        }
    }
    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self, area);
    }
}

impl Widget for &HelpBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(
            Table::new(
                vec![
                    Row::new(["q", "quit", "enter the quit screen."]),
                    Row::new(["o", "open", "enter the chat selection screen."]),
                    Row::new(["u", "users sidebar", "Toggle whether the users are shown in a chat sidebar. Available in reading mode."]),

                    Row::new(["?", "help", "enter this help screen."]),
                    Row::new([
                        "m",
                        "mark as read",
                        "mark current chat as read, when in reading mode.",
                    ]),
                    Row::new([
                        "(e|i)",
                        "edit",
                        "enter the editing screen, when in reading mode.",
                    ]),
                    Row::new([
                        "ESC",
                        "leave Mode",
                        "leave help, opening, or editing mode to return to reading mode",
                    ]),
                    Row::new([
                        "Enter",
                        "send/select",
                        "Send Message, when in edit mode. Select chat when in opening mode.",
                    ]),
                ],
                [
                    Constraint::Length(5),
                    Constraint::Length(20),
                    Constraint::Min(10),
                ],
            )
            .column_spacing(1)
            .style(self.default)
            .header(Row::new(vec!["Key", "Name", "Behavior"]).style(self.table_header))
            .block(Block::default())
            .row_highlight_style(self.default_highlight)
            .highlight_spacing(HighlightSpacing::Never),
            area,
            buf,
        );
    }
}
