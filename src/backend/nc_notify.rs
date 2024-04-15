use crate::config::{self};
use notify_rust::{Hint, Notification, Timeout};

#[derive(Debug, Clone)]
pub struct NCNotify {
    app_name: String,
    timout_ms: u32,
    presistent: bool,
    silent: bool,
}

impl NCNotify {
    pub fn new() -> Self {
        NCNotify {
            app_name: config::get().data.general.chat_server_name.clone(),
            timout_ms: config::get().data.notifications.timeout_ms,
            presistent: config::get().data.notifications.persistent,
            silent: config::get().data.notifications.silent,
        }
    }

    pub fn unread_message(
        &self,
        room_name: &String,
        number_of_unread: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut notification = Notification::new()
            .summary(format!("Unread: {room_name}").as_str())
            .body(format!("You have {number_of_unread} new Messages in {room_name}").as_str())
            .icon("dialog-information")
            .appname(self.app_name.as_str())
            .to_owned();
        if self.presistent {
            log::debug!("Persistent Message!");
            notification
                .hint(Hint::Resident(true)) // this is not supported by all implementations
                .timeout(Timeout::Never); // this however is
        } else {
            notification.timeout(Timeout::Milliseconds(self.timout_ms));
        }
        notification.hint(Hint::SuppressSound(self.silent));

        notification.show()?;
        Ok(())
    }

    pub fn new_room(&self, room_name: &String) -> Result<(), Box<dyn std::error::Error>> {
        let mut notification = Notification::new()
            .summary(format!("New Room: {room_name}").as_str())
            .body(format!("You have been added to a new Room {room_name}").as_str())
            .icon("dialog-information")
            .appname(self.app_name.as_str())
            .to_owned();
        if self.presistent {
            notification
                .hint(Hint::Resident(true)) // this is not supported by all implementations
                .timeout(Timeout::Never); // this however is
        } else {
            notification.timeout(Timeout::Milliseconds(self.timout_ms));
        }
        notification.hint(Hint::SuppressSound(self.silent));

        notification.show()?;
        Ok(())
    }
}

impl Default for NCNotify {
    fn default() -> Self {
        Self::new()
    }
}
