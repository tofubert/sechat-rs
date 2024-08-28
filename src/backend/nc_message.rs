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
        let time: DateTime<Local> = DateTime::from(
            DateTime::<Utc>::from_timestamp(self.0.timestamp, 0)
                .expect("cannot convert UTC time stamp"),
        );
        time.format("%H:%M").to_string()
    }

    pub fn get_name(&self) -> &str {
        &self.0.actorDisplayName
    }

    pub fn get_message(&self) -> &str {
        &self.0.message
    }

    pub fn get_reactions_str(&self) -> String {
        self.0
            .reactions
            .iter()
            .map(|(icon, number)| format!("('{icon}' times {}), ", &number.to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn get_id(&self) -> i32 {
        self.0.id
    }

    /// return inner data message
    pub fn data(&self) -> &NCReqDataMessage {
        &self.0
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
        self.is_system() && self.0.systemMessage == "message_edited"
    }

    pub fn is_reaction(&self) -> bool {
        self.is_system() && self.0.systemMessage == "reaction"
    }

    pub fn is_command(&self) -> bool {
        self.0.messageType == "command"
    }

    pub fn has_reactions(&self) -> bool {
        !self.0.reactions.is_empty()
    }
}
