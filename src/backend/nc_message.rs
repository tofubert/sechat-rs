use crate::backend::nc_request::NCReqDataMessage;
use chrono::prelude::*;

#[derive(Debug)]
pub struct NCMessage(NCReqDataMessage);

impl From<NCReqDataMessage> for NCMessage {
    fn from(data: NCReqDataMessage) -> Self {
        NCMessage(data)
    }
}

impl NCMessage {
    pub fn get_time_str(&self) -> String {
        let time: DateTime<Local> =
            DateTime::from(DateTime::<Utc>::from_timestamp(self.0.timestamp, 0).unwrap());
        time.format("%H:%M").to_string()
    }

    pub fn get_name(&self) -> String {
        self.0.actorDisplayName.clone()
    }

    pub fn get_message(&self) -> &str {
        &self.0.message
    }

    pub fn get_reactions_str(&self) -> String {
        let mut reactions = String::new();
        for (icon, number) in &self.0.reactions {
            reactions = reactions + "('" + icon + "' times " + &number.to_string() + "), ";
        }
        reactions
    }

    pub fn get_id(&self) -> i32 {
        self.0.id
    }

    pub fn to_data(&self) -> NCReqDataMessage {
        self.0.clone()
    }

    pub fn is_comment(&self) -> bool {
        self.0.messageType == "comment"
    }

    pub fn is_comment_deleted(&self) -> bool {
        self.0.messageType == "comment_deleted"
    }

    pub fn is_system(&self) -> bool {
        self.0.messageType == "system"
    }

    pub fn is_edit_note(&self) -> bool {
        if self.is_system() {
            self.0.systemMessage == "message_edited"
        } else {
            false
        }
    }

    pub fn is_reaction(&self) -> bool {
        if self.is_system() {
            self.0.systemMessage == "reaction"
        } else {
            false
        }
    }

    pub fn is_command(&self) -> bool {
        self.0.messageType == "command"
    }

    pub fn has_reactions(&self) -> bool {
        !self.0.reactions.is_empty()
    }
}
