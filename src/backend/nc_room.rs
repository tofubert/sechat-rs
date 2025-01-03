use super::{
    nc_message::NCMessage,
    nc_notify::NCNotify,
    nc_request::{NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom, NCRequestInterface},
};
use async_trait::async_trait;
use log;
use num_derive::FromPrimitive;
use num_traits::{AsPrimitive, FromPrimitive};
use std::fmt::{Debug, Display};

#[derive(Debug, FromPrimitive, PartialEq, Default)]
pub enum NCRoomTypes {
    #[default]
    OneToOne = 1,
    Group,
    Public,
    ChangeLog,
    Deprecated,
    NoteToSelf,
}

#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait NCRoomInterface: Debug + Send + Display + Ord + Default {
    fn get_last_room_level_message_id(&self) -> Option<i32>;
    fn has_unread(&self) -> bool;
    #[allow(dead_code)]
    fn is_dm(&self) -> bool;
    fn is_group(&self) -> bool;
    fn get_messages(&self) -> &Vec<NCMessage>;
    fn get_unread(&self) -> usize;
    fn get_display_name(&self) -> &str;
    fn get_last_read(&self) -> i32;
    fn get_users(&self) -> &Vec<NCReqDataParticipants>;
    fn get_room_type(&self) -> &NCRoomTypes;

    #[allow(dead_code)]
    fn to_json(&self) -> String;
    fn to_data(&self) -> NCReqDataRoom;
    fn write_to_log(&mut self) -> Result<(), std::io::Error>;
    fn to_token(&self) -> String;
    async fn update_if_id_is_newer(
        &mut self,
        message_id: i32,
        data_option: Option<NCReqDataRoom>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn send(&self, message: String) -> Result<String, Box<dyn std::error::Error>>;
    async fn update(
        &mut self,
        data_option: Option<NCReqDataRoom>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn mark_as_read(&self) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Debug, Default)]
pub struct NCRoom<Requester: NCRequestInterface + 'static + std::marker::Sync> {
    requester: Requester,
    notifier: NCNotify,
    pub messages: Vec<NCMessage>,
    room_data: NCReqDataRoom,
    path_to_log: std::path::PathBuf,
    pub room_type: NCRoomTypes,
    participants: Vec<NCReqDataParticipants>,
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> NCRoom<Requester> {
    pub async fn new(
        room_data: NCReqDataRoom,
        requester: Requester,
        notifier: NCNotify,
        path_to_log: std::path::PathBuf,
    ) -> Option<NCRoom<Requester>> {
        let mut tmp_path_buf = path_to_log.clone();
        tmp_path_buf.push(room_data.token.as_str());
        let path = tmp_path_buf.as_path();

        let mut messages = Vec::<NCMessage>::new();

        if path.exists() && path.is_file() {
            if let Ok(data) = serde_json::from_str::<Vec<NCReqDataMessage>>(
                std::fs::read_to_string(path).unwrap().as_str(),
            ) {
                messages.extend(data.into_iter().map(Into::into));
            } else {
                log::debug!(
                    "Failed to parse json for {}, falling back to fetching",
                    room_data.displayName
                );
                NCRoom::<Requester>::fetch_messages(&requester, &room_data.token, &mut messages)
                    .await
                    .ok();
            }
        } else {
            log::debug!("No Log File found for room {}", room_data.displayName);
            NCRoom::<Requester>::fetch_messages(&requester, &room_data.token, &mut messages)
                .await
                .ok();
        }
        let participants = requester
            .fetch_participants(&room_data.token)
            .await
            .expect("Failed to fetch room participants");

        Some(NCRoom {
            requester,
            notifier,
            messages,
            path_to_log: tmp_path_buf,
            room_type: FromPrimitive::from_i32(room_data.roomtype).unwrap(),
            participants,
            room_data,
        })
    }
    async fn fetch_messages(
        requester: &Requester,
        token: &str,
        messages: &mut Vec<NCMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = requester.fetch_chat_initial(token, 200).await?;
        for message in response {
            messages.push(message.into());
        }
        Ok(())
    }
}

#[async_trait]
impl<Requester: NCRequestInterface + 'static + std::marker::Sync> NCRoomInterface
    for NCRoom<Requester>
{
    // the room endpoint doesnt tell you about reactions...
    fn get_last_room_level_message_id(&self) -> Option<i32> {
        self.messages
            .iter()
            .filter(|&message| !message.is_reaction() && !message.is_edit_note())
            .collect::<Vec<&NCMessage>>()
            .last()
            .map(|message| message.get_id())
    }

    fn has_unread(&self) -> bool {
        self.room_data.unreadMessages > 0
    }

    fn is_dm(&self) -> bool {
        match self.room_type {
            NCRoomTypes::OneToOne | NCRoomTypes::NoteToSelf | NCRoomTypes::ChangeLog => true,
            NCRoomTypes::Deprecated | NCRoomTypes::Group | NCRoomTypes::Public => false,
        }
    }

    fn is_group(&self) -> bool {
        match self.room_type {
            NCRoomTypes::Deprecated
            | NCRoomTypes::OneToOne
            | NCRoomTypes::NoteToSelf
            | NCRoomTypes::ChangeLog => false,
            NCRoomTypes::Group | NCRoomTypes::Public => true,
        }
    }

    fn get_room_type(&self) -> &NCRoomTypes {
        &self.room_type
    }

    fn get_messages(&self) -> &Vec<NCMessage> {
        &self.messages
    }

    fn get_unread(&self) -> usize {
        self.room_data.unreadMessages.as_()
    }

    fn get_display_name(&self) -> &str {
        &self.room_data.displayName
    }

    fn get_last_read(&self) -> i32 {
        self.room_data.lastReadMessage
    }
    fn get_users(&self) -> &Vec<NCReqDataParticipants> {
        &self.participants
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&self.room_data).unwrap()
    }

    fn to_data(&self) -> NCReqDataRoom {
        self.room_data.clone()
    }

    fn write_to_log(&mut self) -> Result<(), std::io::Error> {
        use std::io::Write;

        let data: Vec<_> = self.messages.iter().map(NCMessage::data).collect();
        let path = self.path_to_log.as_path();
        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = match std::fs::File::create(path) {
            Err(why) => {
                log::warn!(
                    "Couldn't create log file {} for {}: {}",
                    path.to_str().unwrap(),
                    self.room_data.displayName,
                    why
                );
                return Err(why);
            }
            Ok(file) => file,
        };

        match file.write_all(serde_json::to_string(&data).unwrap().as_bytes()) {
            Err(why) => {
                log::warn!(
                    "couldn't write log file to {} for {}: {}",
                    path.as_os_str()
                        .to_str()
                        .expect("Could not convert log path to string"),
                    self.room_data.displayName,
                    why
                );
                Err(why)
            }
            Ok(()) => Ok(()),
        }
    }

    fn to_token(&self) -> String {
        self.room_data.token.clone()
    }

    async fn send(&self, message: String) -> Result<String, Box<dyn std::error::Error>> {
        log::debug!("Send Message {}", &message);
        let response = self
            .requester
            .send_message(message, self.room_data.token.as_str())
            .await;
        match response {
            Ok(v) => Ok(v.message),
            Err(v) => Err(v),
        }
    }

    async fn update(
        &mut self,
        data_option: Option<NCReqDataRoom>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(data) = data_option {
            self.room_data = data.clone();
        }
        let response = self
            .requester
            .fetch_chat_update(
                self.room_data.token.as_str(),
                200,
                self.messages.last().unwrap().get_id(),
            )
            .await
            .unwrap();
        if self.has_unread() && !response.is_empty() {
            self.notifier
                .unread_message(&self.room_data.displayName, response.len())?;
        }
        if !response.is_empty() {
            log::debug!(
                "Updating {} adding {} new Messages",
                self.to_string(),
                response.len().to_string()
            );
        }
        for message in response {
            self.messages.push(message.into());
        }
        self.participants = self
            .requester
            .fetch_participants(&self.room_data.token)
            .await
            .expect("Failed to fetch room participants");

        Ok(())
    }
    async fn mark_as_read(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.messages.is_empty() {
            self.requester
                .mark_chat_read(
                    &self.room_data.token,
                    self.messages.last().ok_or("No last message")?.get_id(),
                )
                .await?;
        }
        Ok(())
    }
    async fn update_if_id_is_newer(
        &mut self,
        message_id: i32,
        data_option: Option<NCReqDataRoom>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::cmp::Ordering;

        if let Some(last_internal_id) = self.get_last_room_level_message_id() {
            match message_id.cmp(&last_internal_id) {
                Ordering::Greater => {
                    log::info!(
                        "New Messages for '{}' was {} now {}",
                        self.to_string(),
                        last_internal_id,
                        message_id
                    );
                    self.update(data_option).await?;
                }
                Ordering::Less => {
                    log::warn!(
                        "Message Id was older than message stored '{}'! Stored {} Upstream {}",
                        self.to_string(),
                        last_internal_id,
                        message_id
                    );
                }
                Ordering::Equal => (),
            }
        }
        Ok(())
    }
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> Ord for NCRoom<Requester> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(other)
    }
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> PartialOrd for NCRoom<Requester> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> PartialEq for NCRoom<Requester> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> Eq for NCRoom<Requester> {}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> std::fmt::Display
    for NCRoom<Requester>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<Requester: NCRequestInterface + 'static + std::marker::Sync> std::ops::Deref
    for NCRoom<Requester>
{
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.room_data.displayName
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static BUTZ: &str = "Butz";
    impl std::ops::Deref for MockNCRoomInterface {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            BUTZ
        }
    }
    impl Ord for MockNCRoomInterface {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.to_string().cmp(&other.to_string())
        }
    }

    impl PartialOrd for MockNCRoomInterface {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.to_string().cmp(&other.to_string()))
        }
    }

    impl PartialEq for MockNCRoomInterface {
        fn eq(&self, other: &Self) -> bool {
            self.to_string() == other.to_string()
        }
    }

    impl Eq for MockNCRoomInterface {}
    impl std::fmt::Display for MockNCRoomInterface {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let self_name = BUTZ.to_string();
            write!(f, "{self_name}")
        }
    }
}
