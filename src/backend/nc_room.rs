use super::{
    nc_message::NCMessage,
    nc_request::{
        nc_requester::NCRequestInterface, NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom,
        Token,
    },
};
use async_trait::async_trait;
use itertools::Itertools;
use log;
use num_derive::FromPrimitive;
use num_traits::{AsPrimitive, FromPrimitive};
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Different Types of rooms defined by the [NC API](https://nextcloud-talk.readthedocs.io/en/latest/constants/#conversation-types)
#[derive(Debug, FromPrimitive, PartialEq, Default)]
pub enum NCRoomTypes {
    /// DM
    #[default]
    OneToOne = 1,
    /// Group Chat
    Group,
    /// Public Channel
    Public,
    /// NC API Change Log
    ChangeLog,
    /// Old One to One
    Deprecated,
    /// Talk to yourself
    NoteToSelf,
}

#[cfg(test)]
use mockall::{automock, predicate::*};

/// Room Interface Trait
/// Holds all public functions for operations on NC Talk Rooms. For details see [NCRoom].
#[cfg_attr(test, automock)]
#[async_trait]
pub trait NCRoomInterface: Debug + Send + Display + Ord + Default {
    /// Get the ID of the last message of this Room.
    /// This is filtered to not include reactions and deleted messages.
    fn get_last_room_level_message_id(&self) -> Option<i32>;
    /// Check if this Room has unread messages.
    fn has_unread(&self) -> bool;
    /// Check if this Room is a DM Room.
    #[allow(dead_code)]
    fn is_dm(&self) -> bool;
    /// Check if this Room is a Group Chat.
    fn is_group(&self) -> bool;
    /// Get a Vector of all the messages in the room.
    fn get_messages(&self) -> &BTreeMap<i32, NCMessage>;
    /// Get how many messages are unread.
    fn get_unread(&self) -> usize;
    /// Check if this Room is a favorite.
    fn is_favorite(&self) -> bool;
    /// Get the human readable display name of the room.
    fn get_display_name(&self) -> &str;
    /// Get the if of the last read messages.
    fn get_last_read(&self) -> i32;
    /// Get a Vector of the users in the Room.
    fn get_users(&self) -> &Vec<NCReqDataParticipants>;
    /// Get the room type.
    fn get_room_type(&self) -> &NCRoomTypes;

    /// Make this room a json object which can be serialised.
    #[allow(dead_code)]
    fn to_json(&self) -> String;
    /// Get the Underlying Data Object of this Room.
    fn to_data(&self) -> NCReqDataRoom;
    /// Write this room into a log file.
    fn write_to_log(&mut self) -> Result<(), std::io::Error>;
    /// Get the rooms token.
    fn to_token(&self) -> Token;
    /// Check if the message ID is newer than the stored one and update the content.
    /// This is needed since the NCTalk will fetch all rooms and only get the overview data.
    async fn update_if_id_is_newer<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        message_id: i32,
        data_option: Option<NCReqDataRoom>,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// Send a Message to this room.
    async fn send<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &self,
        message: String,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<String, Box<dyn std::error::Error>>;
    /// Update this Room.
    async fn update<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        data_option: Option<NCReqDataRoom>,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<Option<(String, usize)>, Box<dyn std::error::Error>>;
    /// Marks this Room as read.
    async fn mark_as_read<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &self,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn fill_history<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Real implementation of the `NCRoom`.
/// Holds its Messages, Participants, Raw Data and Path to write its log to.
#[derive(Debug, Default)]
pub struct NCRoom {
    /// ``BTreeMap`` of all its messages.
    pub messages: BTreeMap<i32, NCMessage>,
    /// Raw Data of this Room.
    room_data: NCReqDataRoom,
    /// Path to write json output to.
    path_to_log: std::path::PathBuf,
    /// Type of this Room.
    pub room_type: NCRoomTypes,
    /// Vec of all Participants in this Room.
    participants: Vec<NCReqDataParticipants>,
}

impl NCRoom {
    /// Create a new `NCRoom`.
    /// Tries to read chat data from the disk, else fetches it.
    /// Requester is in a Thread safe Arc/Mutex.
    pub async fn new<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        room_data: NCReqDataRoom,
        requester: Arc<Mutex<Requester>>,
        path_to_log: std::path::PathBuf,
    ) -> Option<NCRoom> {
        let mut tmp_path_buf = path_to_log.clone();
        tmp_path_buf.push(room_data.token.as_str());
        let path = tmp_path_buf.as_path();

        let mut messages = BTreeMap::<i32, NCMessage>::new();

        if path.exists() && path.is_file() {
            if let Ok(data) = serde_json::from_str::<Vec<NCReqDataMessage>>(
                std::fs::read_to_string(path).unwrap().as_str(),
            ) {
                for message in data {
                    messages.insert(message.id, message.into());
                }
            } else {
                log::debug!(
                    "Failed to parse json for {}, falling back to fetching",
                    room_data.displayName
                );
                NCRoom::fetch_messages::<Requester>(
                    requester.clone(),
                    &room_data.token,
                    &mut messages,
                )
                .await
                .ok();
            }
        } else {
            log::debug!("No Log File found for room {}", room_data.displayName);
            NCRoom::fetch_messages::<Requester>(requester.clone(), &room_data.token, &mut messages)
                .await
                .ok();
        }

        Some(NCRoom {
            messages,
            path_to_log: tmp_path_buf,
            room_type: FromPrimitive::from_i32(room_data.roomtype).unwrap(),
            participants: vec![],
            room_data,
        })
    }
    async fn fetch_messages<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        requester: Arc<Mutex<Requester>>,
        token: &Token,
        messages: &mut BTreeMap<i32, NCMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response_onceshot = {
            requester
                .lock()
                .await
                .request_chat_initial(token, 200)
                .await
                .unwrap()
        };
        let response = response_onceshot
            .await
            .expect("Failed for fetch chat update")
            .expect("Failed request");
        for message in response {
            messages.insert(message.id, message.into());
        }
        Ok(())
    }

    async fn fetch_message_subset<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        first: i32,
        last: i32,
        requester: Arc<Mutex<Requester>>,
        token: &Token,
    ) -> BTreeMap<i32, NCMessage> {
        let mut fetch_key = first;
        let mut messages = BTreeMap::new();
        while fetch_key <= last && fetch_key >= 0 {
            let response_onceshot = {
                requester
                    .lock()
                    .await
                    .request_chat_update(token, 200, fetch_key)
                    .await
                    .unwrap()
            };
            if let Ok(Ok(response)) = response_onceshot.await {
                if response.is_empty() {
                    log::debug!("No Messages found aborting {fetch_key}");
                    break;
                }
                fetch_key = response.last().expect("No Messages fetched").id;

                for message in response {
                    messages.insert(message.id, message.into());
                }
            } else {
                break;
            }
        }
        messages
    }
}

#[async_trait]
impl NCRoomInterface for NCRoom {
    fn get_last_room_level_message_id(&self) -> Option<i32> {
        // the room endpoint doesnt tell you about reactions...
        // The NCTalk level update does however include system messages...
        self.messages
            .values()
            .filter(|&message| {
                !message.is_reaction() && !message.is_edit_note() && !message.is_revoked()
            })
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

    fn get_messages(&self) -> &BTreeMap<i32, NCMessage> {
        &self.messages
    }

    fn get_unread(&self) -> usize {
        self.room_data.unreadMessages.as_()
    }

    fn is_favorite(&self) -> bool {
        self.room_data.isFavorite
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

        let data: Vec<_> = self.messages.values().map(NCMessage::data).collect();
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

    fn to_token(&self) -> Token {
        self.room_data.token.clone()
    }

    async fn send<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &self,
        message: String,
        requester: Arc<Mutex<Requester>>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        log::info!("Send Message {}", &message);
        let response_onceshot = {
            requester
                .lock()
                .await
                .request_send_message(message, &self.room_data.token)
                .await
                .unwrap()
        };
        let response = response_onceshot
            .await
            .expect("Failed for fetch chat participants");
        match response {
            Ok(v) => Ok(v.message),
            Err(why) => Err(why.into()),
        }
    }

    async fn update<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        data_option: Option<NCReqDataRoom>,
        requester: Arc<Mutex<Requester>>,
    ) -> Result<Option<(String, usize)>, Box<dyn std::error::Error>> {
        log::trace!("Updating {}", self.room_data.displayName);
        if let Some(data) = data_option {
            self.room_data = data.clone();
        }
        let response_onceshot = {
            requester
                .lock()
                .await
                .request_chat_update(
                    &self.room_data.token,
                    200,
                    self.messages
                        .get(
                            self.messages
                                .keys()
                                .sorted()
                                .next_back()
                                .expect("Failed to sort messages by its keys."),
                        )
                        .ok_or("No last message")?
                        .get_id(),
                )
                .await
                .unwrap()
        };
        let response = response_onceshot
            .await
            .expect("Failed for fetch chat update")
            .expect("Failed request");

        let is_empty = response.is_empty();
        let update_info = Some((self.room_data.displayName.clone(), response.len()));

        if !is_empty {
            log::info!("Updating {} adding {} new Messages", self, response.len());
        }
        for message in response {
            self.messages.insert(message.id, message.into());
        }
        let response_onceshot = {
            requester
                .lock()
                .await
                .request_participants(&self.room_data.token)
                .await
                .unwrap()
        };

        self.participants = response_onceshot
            .await
            .expect("Failed for fetch chat participants")
            .expect("Failed request");
        if self.has_unread() && !is_empty {
            Ok(update_info)
        } else {
            Ok(None)
        }
    }
    async fn mark_as_read<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &self,
        requester: Arc<Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.messages.is_empty() {
            log::info!("Marking room {} as read", self.room_data.displayName);
            let response_onceshot = {
                requester
                    .lock()
                    .await
                    .request_mark_chat_read(
                        &self.room_data.token,
                        self.messages
                            .get(
                                self.messages
                                    .keys()
                                    .sorted()
                                    .next_back()
                                    .expect("Failed to sort messages by its keys."),
                            )
                            .ok_or("No last message")?
                            .get_id(),
                    )
                    .await
                    .unwrap()
            };
            response_onceshot
                .await
                .expect("Failed for fetch chat participants")
                .expect("Failed request");
        }
        Ok(())
    }
    async fn update_if_id_is_newer<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        message_id: i32,
        data_option: Option<NCReqDataRoom>,
        requester: Arc<Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::cmp::Ordering;
        if let Some(room) = data_option {
            if room.unreadMessages != self.room_data.unreadMessages {
                self.update(Some(room), requester).await?;
            }
        } else if let Some(last_internal_id) = self.get_last_room_level_message_id() {
            match message_id.cmp(&last_internal_id) {
                Ordering::Greater => {
                    let name = self.to_string();
                    log::info!("New Messages for '{name}' was {last_internal_id} now {message_id}");
                    self.update(data_option, requester).await?;
                }
                Ordering::Less => {
                    log::debug!(
                        "Message Id was older than message stored '{}'! Stored {} {} {} Upstream {}",
                        self,
                        last_internal_id,
                        self.messages
                            .get(&last_internal_id)
                            .unwrap()
                            .data()
                            .messageType,
                        self.messages
                            .get(&last_internal_id)
                            .unwrap()
                            .data()
                            .systemMessage,
                        message_id
                    );
                }
                Ordering::Equal => (),
            }
        }

        Ok(())
    }

    async fn fill_history<Requester: NCRequestInterface + 'static + std::marker::Sync>(
        &mut self,
        requester: Arc<tokio::sync::Mutex<Requester>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response_onceshot = {
            requester
                .lock()
                .await
                .request_chat_update(&self.room_data.token, 200, 1)
                .await
                .unwrap()
        };
        let response = response_onceshot
            .await
            .expect("Failed for fetch chat update")
            .expect("Failed request");

        // Room is empty.
        if response.is_empty() {
            return Ok(());
        }

        let fetch_key = response
            .first()
            .expect("Failed to get last message of initial fetch")
            .id;

        let last_entry = *(self
            .messages
            .last_key_value()
            .expect("Failed to sort messages by its keys.")
            .0);

        log::debug!(
            "Fetching Full history. Size stored {}, size to check {}, last {}, first {}",
            self.messages.len(),
            last_entry - fetch_key,
            last_entry,
            fetch_key
        );
        let mut running_key = fetch_key + 10_000;
        let mut thread_handles = vec![];
        for key in (fetch_key..=last_entry).step_by(10_000) {
            log::debug!("Fetching thread {key} to {running_key} ");
            let token = self.room_data.token.clone();
            let cloned_requester = requester.clone();
            thread_handles.push(tokio::spawn(async move {
                NCRoom::fetch_message_subset(key, running_key, cloned_requester, &token).await
            }));
            running_key += 10_000;
        }
        log::debug!("Spawned all reads for fetching");
        for handle in thread_handles {
            let mut results = handle.await.expect("No Messages could be fetched.");
            self.messages.append(&mut results);
        }

        log::debug!(
            "Updated Full history. Size {}, last {}, first {}",
            self.messages.len(),
            last_entry,
            fetch_key
        );

        Ok(())
    }
}

impl Ord for NCRoom {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(other)
    }
}

impl PartialOrd for NCRoom {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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
