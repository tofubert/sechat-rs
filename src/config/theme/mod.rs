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
