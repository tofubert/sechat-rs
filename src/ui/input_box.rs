use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct InputBox {
    current_text: String,
}

impl InputBox {
    pub fn new(initial_message: &str) -> InputBox {
        InputBox {
            current_text: initial_message.to_string(),
        }
    }

    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self, area);
    }
}

impl Widget for &InputBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text: Vec<Line> = textwrap::wrap(
            ("> ".to_string() + &self.current_text).as_str(),
            area.width as usize,
        )
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .map(Line::from)
        .collect();
        Paragraph::new(text)
            .block(Block::default().borders(Borders::TOP))
            .style(Style::new().white().on_black())
            .alignment(Alignment::Left)
            .render(area, buf);
    }
}

impl std::ops::Deref for InputBox {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.current_text
    }
}

impl std::ops::DerefMut for InputBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current_text
    }
}
