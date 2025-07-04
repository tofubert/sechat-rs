use color_hash::color_hash_hex;
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Default)]
pub struct UserStyles {
    /// Map between user and style
    pub user_style_map: HashMap<String, Style>,
    /// Styles to allocate to users
    pub styles: Vec<Style>,
    /// Index of next style to allocate
    pub styles_index: usize,
    /// Path to use for serialization/deserialization
    #[serde(skip)]
    pub data_log_path: PathBuf,
}

impl UserStyles {
    pub fn new(data_log_path: &Path) -> Self {
        // First try to read data from saved data
        let mut tmp_data_log_path = data_log_path.to_path_buf();
        tmp_data_log_path.push("UserStyles.json");
        let path = tmp_data_log_path.as_path();
        if path.exists() {
            let buf_reader = BufReader::new(File::open(path).unwrap());
            let mut user_styles: Self = serde_json::from_reader(buf_reader).unwrap();
            user_styles.data_log_path = tmp_data_log_path;
            return user_styles;
        }

        let mut styles = Vec::new();
        UserStyles::generate_new_styles(&mut styles);
        Self {
            user_style_map: HashMap::<String, Style>::new(),
            styles,
            styles_index: 0,
            data_log_path: tmp_data_log_path,
        }
    }

    pub fn update(&mut self, user_id: &str) {
        if !self.user_style_map.contains_key(user_id) {
            log::trace!("added {user_id}");
            self.user_style_map.insert(
                user_id.to_string(),
                *self.styles.get(self.styles_index).expect("style not found"),
            );
            self.styles_index += 1;
            if self.styles_index >= self.styles.len() {
                UserStyles::generate_new_styles(&mut self.styles);
            }
        }
    }

    pub fn write_to_log(&self) -> Result<(), std::io::Error> {
        let path = self.data_log_path.as_path();
        let file = match std::fs::File::create(path) {
            Err(why) => {
                log::error!(
                    "couldn't create user_styles log file {}: {}",
                    path.as_os_str()
                        .to_str()
                        .expect("Path didn't become string"),
                    why,
                );
                return Err(why);
            }
            Ok(file) => file,
        };
        let buf_writer = BufWriter::new(file);
        serde_json::to_writer(buf_writer, &self)?;
        Ok(())
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
        let mut user_styles = UserStyles::new(&PathBuf::new());
        let user_id_0 = "Terror";
        user_styles.update(user_id_0);
        let user_id_1 = "Licky";
        user_styles.update(user_id_1);

        assert_eq!(
            user_styles.user_style_map.get(user_id_0).unwrap(),
            user_styles.styles.first().unwrap(),
        );
        assert_eq!(
            user_styles.user_style_map.get(user_id_1).unwrap(),
            user_styles.styles.get(1).unwrap(),
        );
    }
}
