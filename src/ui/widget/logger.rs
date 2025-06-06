use crate::config::Config;
use log::LevelFilter;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiWidgetEvent, TuiWidgetState};

#[derive(Default)]
pub struct LogBox {
    state: TuiWidgetState,
    style: Style,
}

impl LogBox {
    pub fn new(config: &Config) -> Self {
        LogBox {
            state: TuiWidgetState::new().set_default_display_level(LevelFilter::Debug),
            style: config.theme.default_style(),
        }
    }
    pub fn handle_ui_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(' ') => self.state.transition(TuiWidgetEvent::SpaceKey),
            KeyCode::PageUp => self.state.transition(TuiWidgetEvent::PrevPageKey),
            KeyCode::PageDown => self.state.transition(TuiWidgetEvent::NextPageKey),
            KeyCode::Up => self.state.transition(TuiWidgetEvent::UpKey),
            KeyCode::Down => self.state.transition(TuiWidgetEvent::DownKey),
            KeyCode::Left => self.state.transition(TuiWidgetEvent::LeftKey),
            KeyCode::Right => self.state.transition(TuiWidgetEvent::RightKey),
            KeyCode::Char('+') => self.state.transition(TuiWidgetEvent::PlusKey),
            KeyCode::Char('-') => self.state.transition(TuiWidgetEvent::MinusKey),
            KeyCode::Char('h') => self.state.transition(TuiWidgetEvent::HideKey),
            KeyCode::Char('f') => self.state.transition(TuiWidgetEvent::FocusKey),
            KeyCode::Char('s') => self.state.transition(TuiWidgetEvent::EscapeKey),
            _ => (),
        }
    }
    pub fn render_area(&self, frame: &mut Frame, area: Rect) {
        let [log_area, help_area] =
            Layout::vertical([Constraint::Fill(50), Constraint::Length(3)]).areas(area);

        let logger = TuiLoggerSmartWidget::default()
            .style_error(self.style.fg(Color::Red))
            .style_debug(self.style.fg(Color::Green))
            .style_warn(self.style.fg(Color::Yellow))
            .style_trace(self.style.fg(Color::Magenta))
            .style_info(self.style.fg(Color::Cyan))
            .style(self.style)
            .output_separator('|')
            .output_timestamp(Some("%H:%M:%S%.3f".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(true)
            .state(&self.state);
        frame.render_widget(logger, log_area);
        if area.width > 40 {
            let help_text = Text::from(vec![
                "s: Cancel Scroll | Tab: Switch state | ↑/↓: Select target | f: Focus target"
                    .into(),
                "←/→: Display level | +/-: Filter level | Space: Toggle hidden targets".into(),
                "h: Hide target selector | PageUp/Down: Scroll | Esc: Exit this screen".into(),
            ])
            .style(self.style)
            .centered();
            frame.render_widget(help_text, help_area);
        }
    }
}
