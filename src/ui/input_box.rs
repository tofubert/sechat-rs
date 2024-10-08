use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
};
use tui_textarea::TextArea;

#[derive(Default)]
pub struct InputBox<'a> {
    textarea: TextArea<'a>,
}

impl<'a> InputBox<'a> {
    pub fn new(initial_message: &str) -> InputBox<'a> {
        let mut textarea = TextArea::new(vec![initial_message.into()]);
        textarea.set_block(Block::default().borders(Borders::TOP));
        InputBox { textarea }
    }

    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(&self.textarea, area);
    }
}

impl<'a> std::ops::Deref for InputBox<'a> {
    type Target = TextArea<'a>;

    fn deref(&self) -> &Self::Target {
        &self.textarea
    }
}

impl<'a> std::ops::DerefMut for InputBox<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.textarea
    }
}
