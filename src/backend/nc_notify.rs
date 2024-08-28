use crate::config::{self};
use notify_rust::{Hint, Notification, Timeout};

#[derive(Debug, Clone, Default)]
pub struct NCNotify {
    app_name: String,
    timeout: Timeout,
    silent: bool,
}

impl NCNotify {
    pub fn new() -> Self {
        let data = &config::get().data;
        NCNotify {
            app_name: data.general.chat_server_name.clone(),
            timeout: if data.notifications.persistent {
                Timeout::Never
            } else {
                Timeout::Milliseconds(data.notifications.timeout_ms)
            },
            silent: data.notifications.silent,
        }
    }

    pub fn unread_message(
        &self,
        room_name: &String,
        number_of_unread: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut notification = Notification::new()
            .summary(&format!("Unread: {room_name}"))
            .body(&format!(
                "You have {number_of_unread} new Messages in {room_name}"
            ))
            .icon("dialog-information")
            .appname(&self.app_name)
            .to_owned();
        if self.is_persistent() {
            log::debug!("Persistent Message!");
        }
        notification
            .hint(Hint::Resident(self.is_persistent())) // this is not supported by all implementations
            .timeout(self.timeout);
        notification.hint(Hint::SuppressSound(self.silent));

        notification.show()?;
        Ok(())
    }

    pub fn new_room(&self, room_name: &String) -> Result<(), Box<dyn std::error::Error>> {
        let mut notification = Notification::new()
            .summary(&format!("New Room: {room_name}"))
            .body(&format!("You have been added to a new Room {room_name}"))
            .icon("dialog-information")
            .appname(&self.app_name)
            .to_owned();
        notification
            .hint(Hint::Resident(self.is_persistent())) // this is not supported by all implementations
            .timeout(self.timeout); // this however is
        notification.hint(Hint::SuppressSound(self.silent));

        notification.show()?;
        Ok(())
    }

    /// return `true` if notification is persistent (has infinite display timeout)
    pub fn is_persistent(&self) -> bool {
        self.timeout == Timeout::Never
    }
}
