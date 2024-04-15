use ratatui::{
    prelude::*,
    widgets::{Block, HighlightSpacing, Row, Table},
};

#[derive(Default)]
pub struct HelpBox {}

impl HelpBox {
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
            .style(Style::new().white().on_black())
            .header(Row::new(vec!["Key", "Name", "Behaviour"]).style(Style::new().bold().blue()))
            .block(Block::default())
            .highlight_style(Style::new().green())
            .highlight_spacing(HighlightSpacing::Never),
            area,
            buf,
        );
    }
}
