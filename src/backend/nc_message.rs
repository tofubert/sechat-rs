use crate::backend::nc_request::NCReqDataMessage;
use chrono::prelude::*;

#[derive(Debug)]
pub struct NCMessage {
    data: NCReqDataMessage,
    message: String,
}

impl From<NCReqDataMessage> for NCMessage {
    fn from(data: NCReqDataMessage) -> Self {
        NCMessage {
            message: data.message.clone(),
            data,
        }
    }
}

impl NCMessage {
    pub fn get_time_str(&self) -> String {
        let time: DateTime<Local> =
            DateTime::from(DateTime::<Utc>::from_timestamp(self.data.timestamp, 0).unwrap());
        time.format("%H:%M").to_string()
    }

    pub fn get_name(&self) -> String {
        self.data.actorDisplayName.clone()
    }

    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    pub fn get_reactions_str(&self) -> String {
        let mut reactions = String::new();
        for (icon, number) in &self.data.reactions {
            reactions = reactions + "('" + icon + "' times " + &number.to_string() + "), ";
        }
        reactions
    }

    pub fn get_id(&self) -> i32 {
        self.data.id
    }

    pub fn to_data(&self) -> NCReqDataMessage {
        self.data.clone()
    }

    pub fn is_comment(&self) -> bool {
        self.data.messageType == "comment"
    }

    pub fn is_comment_deleted(&self) -> bool {
        self.data.messageType == "comment_deleted"
    }

    pub fn is_system(&self) -> bool {
        self.data.messageType == "system"
    }

    pub fn is_edit_note(&self) -> bool {
        if self.is_system() {
            self.data.systemMessage == "message_edited"
        } else {
            false
        }
    }

    pub fn is_reaction(&self) -> bool {
        if self.is_system() {
            self.data.systemMessage == "reaction"
        } else {
            false
        }
    }

    pub fn is_command(&self) -> bool {
        self.data.messageType == "command"
    }

    pub fn has_reactions(&self) -> bool {
        !self.data.reactions.is_empty()
    }
}
