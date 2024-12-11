use ratatui::style::Style;
use ratatui::style::Stylize;

pub mod options;

#[derive(Debug, Default)]
pub struct Theme {
    data: options::ColorPalette,
}

impl Theme {
    pub fn set_theme(&mut self, data: options::ColorPalette) {
        self.data = data;
    }
    pub fn default_style(&self) -> Style {
        Style::new()
            .fg(self.data.foreground)
            .bg(self.data.background)
    }
    pub fn default_highlight_style(&self) -> Style {
        Style::new()
            .fg(self.data.foreground_highlight)
            .bg(self.data.background_highlight)
    }
    pub fn user_away_style(&self) -> Style {
        Style::new()
            .fg(self.data.user_away)
            .bg(self.data.background)
    }
    pub fn user_dnd_style(&self) -> Style {
        Style::new().fg(self.data.user_dnd).bg(self.data.background)
    }
    pub fn user_offline_style(&self) -> Style {
        Style::new()
            .fg(self.data.user_offline)
            .bg(self.data.background)
    }
    pub fn user_online_style(&self) -> Style {
        Style::new()
            .fg(self.data.user_online)
            .bg(self.data.background)
    }
    pub fn unread_message_style(&self) -> Style {
        Style::new()
            .fg(self.data.foreground_unread_message)
            .bg(self.data.background_unread_message)
    }

    pub fn table_header_style(&self) -> Style {
        Style::new()
            .bold()
            .fg(self.data.table_header)
            .bg(self.data.background)
    }

    pub fn title_status_style(&self) -> Style {
        Style::new()
            .bg(self.data.background)
            .fg(self.data.foreground_titlebar)
    }

    pub fn title_important_style(&self) -> Style {
        Style::new()
            .bold()
            .bg(self.data.background_important_titlebar)
            .fg(self.data.foreground_important_titlebar)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;
    use ratatui::style::Style;

    use super::*;

    #[test]
    fn default_values() {
        let theme = Theme::default();
        assert_eq!(
            theme.default_highlight_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.user_away_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.user_dnd_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.user_offline_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.user_online_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.unread_message_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.table_header_style(),
            Style::new()
                .fg(Color::default())
                .bg(Color::default())
                .bold()
        );
        assert_eq!(
            theme.title_status_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
        assert_eq!(
            theme.title_important_style(),
            Style::new()
                .fg(Color::default())
                .bg(Color::default())
                .bold()
        );
    }

    #[test]
    fn set_data() {
        let mut theme = Theme::default();
        theme.set_theme(options::ColorPalette::default());
        assert_eq!(
            theme.unread_message_style(),
            Style::new().fg(Color::default()).bg(Color::default())
        );
    }
}
