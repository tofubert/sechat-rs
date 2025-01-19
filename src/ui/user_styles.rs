use random_color::RandomColor;
use ratatui::style::{Color, Style};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Default)]
pub struct UserStyles {
    /// Map between user and style
    pub user_styles: HashMap<String, Style>,
    /// Styles to allocate to users
    pub styles: Vec<Style>,
    /// Index of next style to allocate
    pub styles_index: usize,
}

impl UserStyles {
    pub fn new() -> Self {
        let mut styles = Vec::new();
        UserStyles::generate_new_styles(&mut styles);
        Self {
            user_styles: HashMap::<String, Style>::new(),
            styles,
            styles_index: 0,
        }
    }

    pub fn update(&mut self, user_id: &str) {
        if !self.user_style.iter().any(|m| m.0.eq(user_id)) {
            log::trace!("added {user_id}");
            self.user_styles.insert(
                user_id.to_string(),
                *self.styles.get(self.styles_index).expect("style not found"),
            );
            self.styles_index += 1;
            if self.styles_index >= self.styles.len() {
                UserStyles::generate_new_styles(&mut self.styles);
            }
        }
    }

    fn generate_new_styles(styles: &mut Vec<Style>) {
        for _ in 0..50 {
            styles.push(
                Color::from_str(RandomColor::new().to_hex().as_str())
                    .expect("not a color")
                    .into(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update() {
        let mut user_styles = UserStyles::new();
        let user_id_0 = "Terror";
        user_styles.update(user_id_0);
        let user_id_1 = "Licky";
        user_styles.update(user_id_1);

        assert_eq!(
            user_styles.user_styles.get(user_id_0).unwrap(),
            user_styles.styles.first().unwrap(),
        );
        assert_eq!(
            user_styles.user_styles.get(user_id_1).unwrap(),
            user_styles.styles.get(1).unwrap(),
        );
    }
}
