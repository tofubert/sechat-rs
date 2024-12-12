use crate::config::Config;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
};
use tui_textarea::TextArea;

#[derive(Default)]
pub struct InputBox<'a> {
    textarea: TextArea<'a>,
}

impl InputBox<'_> {
    pub fn new(initial_message: &str, config: &Config) -> Self {
        let mut textarea = TextArea::new(vec![initial_message.into()]);
        textarea.set_block(
            Block::default()
                .borders(Borders::TOP)
                .style(config.theme.default_style()),
        );
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

impl std::ops::DerefMut for InputBox<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.textarea
    }
}

#[cfg(test)]
mod tests {

    use crate::backend::nc_request::NCReqDataParticipants;
    use crate::config::init;
    use backend::TestBackend;

    use super::*;

    #[test]
    fn render_users() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();

        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut input_box = InputBox::new("test", &config);

        let mut dummy_user = NCReqDataParticipants::default();
        dummy_user.displayName = "Butz".to_string();

        terminal
            .draw(|frame| input_box.render_area(frame, Rect::new(0, 0, 30, 3)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "──────────────────────────────",
            "test                          ",
            "                              ",
        ]);
        expected.set_style(Rect::new(0, 0, 30, 3), config.theme.default_style());

        expected.set_style(
            Rect::new(0, 1, 1, 1),
            config.theme.default_style().reversed(),
        );
        expected.set_style(
            Rect::new(1, 1, 3, 1),
            config.theme.default_style().underlined(),
        );

        terminal.backend().assert_buffer(&expected);

        input_box.insert_char('b');

        terminal
            .draw(|frame| input_box.render_area(frame, Rect::new(0, 0, 30, 3)))
            .unwrap();

        let mut expected = Buffer::with_lines([
            "──────────────────────────────",
            "btest                         ",
            "                              ",
        ]);
        expected.set_style(Rect::new(0, 0, 30, 3), config.theme.default_style());
        expected.set_style(
            Rect::new(2, 1, 3, 1),
            config.theme.default_style().underlined(),
        );
        expected.set_style(
            Rect::new(0, 1, 1, 1),
            config.theme.default_style().underlined(),
        );
        expected.set_style(
            Rect::new(1, 1, 1, 1),
            config.theme.default_style().reversed(),
        );

        terminal.backend().assert_buffer(&expected);
    }
}
