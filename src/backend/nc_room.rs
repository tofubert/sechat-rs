use super::{
    nc_message::NCMessage,
    nc_notify::NCNotify,
    nc_request::{NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom, NCRequest},
};
use log;
use num_derive::FromPrimitive;
use num_traits::{AsPrimitive, FromPrimitive};
use std::{
    cmp::Ordering,
    error::Error,
    fs::{read_to_string, File},
    io::prelude::*,
    path::PathBuf,
};

#[derive(Debug, FromPrimitive, PartialEq)]
pub enum NCRoomTypes {
    OneToOne = 1,
    Group,
    Public,
    ChangeLog,
    Deprecated,
    NoteToSelf,
}

#[derive(Debug)]
pub struct NCRoom {
    requester: NCRequest,
    notifier: NCNotify,
    pub messages: Vec<NCMessage>,
    room_data: NCReqDataRoom,
    path_to_log: PathBuf,
    pub room_type: NCRoomTypes,
    participants: Vec<NCReqDataParticipants>,
}

impl NCRoom {
    async fn fetch_messages(
        requester: NCRequest,
        token: String,
        messages: &mut Vec<NCMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let response = requester
            .fetch_chat_initial(token.clone().as_str(), 200)
            .await?;
        for message in response {
            messages.push(message.into());
        }
        Ok(())
    }

    pub async fn new(
        room_data: NCReqDataRoom,
        requester: NCRequest,
        notifier: NCNotify,
        path_to_log: PathBuf,
    ) -> Option<NCRoom> {
        let mut tmp_path_buf = path_to_log.clone();
        tmp_path_buf.push(room_data.token.as_str());
        let path = tmp_path_buf.as_path();

        let mut messages = Vec::<NCMessage>::new();

        if path.exists() {
            if let Ok(data) = serde_json::from_str::<Vec<NCReqDataMessage>>(
                read_to_string(path).unwrap().as_str(),
            ) {
                messages.extend(data.into_iter().map(Into::into));
            } else {
                log::debug!(
                    "Failed to parse json for {}, falling back to fetching",
                    room_data.displayName
                );
                NCRoom::fetch_messages(requester.clone(), room_data.token.clone(), &mut messages)
                    .await
                    .ok();
            }
        } else {
            log::debug!("No Log File found for room {}", room_data.displayName);
            NCRoom::fetch_messages(requester.clone(), room_data.token.clone(), &mut messages)
                .await
                .ok();
        }

        let type_num = room_data.roomtype;
        let participants = requester
            .fetch_participants(&room_data.token)
            .await
            .expect("Failed to fetch room participants");

        Some(NCRoom {
            requester,
            notifier,
            room_data,
            messages,
            path_to_log: tmp_path_buf,
            room_type: FromPrimitive::from_i32(type_num).unwrap(),
            participants,
        })
    }

    pub async fn send(&self, message: String) -> Result<String, Box<dyn Error>> {
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

    pub async fn update(
        &mut self,
        data_option: Option<&NCReqDataRoom>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(data) = data_option {
            self.room_data = data.clone();
        }
        let response = self
            .requester
            .fetch_chat_update(
                self.room_data.token.clone().as_str(),
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

    // the room endpoint doesnt tell you about reactions...
    pub fn get_last_room_level_message_id(&self) -> Option<i32> {
        self.messages
            .iter()
            .filter(|&message| !message.is_reaction() && !message.is_edit_note())
            .collect::<Vec<&NCMessage>>()
            .last()
            .map(|message| message.get_id())
    }

    pub async fn mark_as_read(&self) -> Result<(), Box<dyn Error>> {
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

    pub fn has_unread(&self) -> bool {
        self.room_data.unreadMessages > 0
    }

    pub fn get_unread(&self) -> usize {
        self.room_data.unreadMessages.as_()
    }

    pub fn get_display_name(&self) -> &str {
        &self.room_data.displayName
    }

    pub fn get_last_read(&self) -> i32 {
        self.room_data.lastReadMessage
    }

    pub async fn update_if_id_is_newer(
        &mut self,
        messageid: i32,
        data_option: Option<&NCReqDataRoom>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(last_internal_id) = self.get_last_room_level_message_id() {
            match messageid.cmp(&last_internal_id) {
                Ordering::Greater => {
                    log::info!(
                        "New Messages for '{}' was {} now {}",
                        self.to_string(),
                        last_internal_id,
                        messageid
                    );
                    self.update(data_option).await?;
                }
                Ordering::Less => {
                    log::warn!(
                        "Message Id was older than message stored '{}'! Stored {} Upstream {}",
                        self.to_string(),
                        last_internal_id,
                        messageid
                    );
                }
                Ordering::Equal => (),
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.room_data).unwrap()
    }

    pub fn to_data(&self) -> NCReqDataRoom {
        self.room_data.clone()
    }

    pub fn write_to_log(&mut self) -> Result<(), std::io::Error> {
        let data: Vec<_> = self.messages.iter().map(NCMessage::data).collect();
        let path = self.path_to_log.as_path();
        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = match File::create(path) {
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

    pub fn to_token(&self) -> String {
        self.room_data.token.clone()
    }
}

impl Ord for NCRoom {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(other)
    }
}

impl PartialOrd for NCRoom {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NCRoom {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for NCRoom {}

impl std::fmt::Display for NCRoom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::ops::Deref for NCRoom {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.room_data.displayName
    }
}
