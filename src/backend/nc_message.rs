use super::nc_request::NCReqDataMessage;
use chrono::prelude::*;

/// `NextCloud` message interface
#[derive(Debug, Default, Clone)]
pub struct NCMessage(NCReqDataMessage);

impl From<NCReqDataMessage> for NCMessage {
    fn from(data: NCReqDataMessage) -> Self {
        NCMessage(data)
    }
}

impl NCMessage {
    /// return message time stamp as string
    pub fn get_time_str(&self) -> String {
        let time: DateTime<Local> = DateTime::from(
            DateTime::<Utc>::from_timestamp(self.0.timestamp, 0)
                .expect("cannot convert UTC time stamp"),
        );
        time.format("%H:%M").to_string()
    }

    pub fn get_date_str(&self, date_format: &str) -> String {
        let date: DateTime<Local> = DateTime::from(
            DateTime::<Utc>::from_timestamp(self.0.timestamp, 0)
                .expect("cannot convert UTC time stamp"),
        );
        date.format(date_format).to_string()
    }

    /// return opponent display name
    pub fn get_name(&self) -> &str {
        if !self.is_comment() || self.is_system() || self.is_comment_deleted() || self.is_command()
        {
            "System"
        } else {
            &self.0.actorDisplayName
        }
    }

    /// return the message itself
    pub fn get_message(&self) -> &str {
        &self.0.message
    }

    /// get list of reactions as comma separated string
    pub fn get_reactions_str(&self) -> String {
        self.0
            .reactions
            .iter()
            .map(|(icon, number)| format!("('{icon}' times {}), ", &number.to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// get message identifier
    pub fn get_id(&self) -> i32 {
        self.0.id
    }

    /// return inner data message
    pub fn data(&self) -> &NCReqDataMessage {
        &self.0
    }

    /// return `true` if message is a comment
    pub fn is_comment(&self) -> bool {
        self.0.messageType == "comment"
    }

    /// return `true` if message is a deleted comment
    pub fn is_comment_deleted(&self) -> bool {
        self.0.messageType == "comment_deleted"
    }

    /// return `true` if message is a system message
    pub fn is_system(&self) -> bool {
        self.0.messageType == "system"
    }

    /// return `true` if message is an edited message
    pub fn is_edit_note(&self) -> bool {
        self.is_system() && self.0.systemMessage == "message_edited"
    }

    /// return `true` if message is a reaction
    pub fn is_reaction(&self) -> bool {
        self.is_system() && self.0.systemMessage == "reaction"
    }

    /// return `true` if message is a command
    pub fn is_command(&self) -> bool {
        self.0.messageType == "command"
    }

    /// return `true` if message has any reactions
    pub fn has_reactions(&self) -> bool {
        !self.0.reactions.is_empty()
    }
}
