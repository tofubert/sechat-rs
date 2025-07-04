//! NC Talk Message Object
//!
//! [`NCTalk`] for the main implementation.
//!
//! [`NCBackend`] as the trait to enable testing.

use crate::{
    backend::{
        nc_request::{nc_requester::NCRequestInterface, NCReqDataRoom},
        nc_room::NCRoomInterface,
    },
    config::Config,
};
use async_trait::async_trait;
use itertools::Itertools;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{sync::Mutex, task::JoinHandle};

use super::{
    nc_request::Token,
    nc_room::{NCRoom, NCRoomTypes},
};

/// Public Trait for NC Talk Instance Object used for all interaction with the server.
///
/// This trait is needed due to the use of the [mockall](https://crates.io/crates/mockall) crate in testing.
/// See [`NCTalk`] for more details on the functionality.
#[async_trait]
pub trait NCBackend: Debug + Send {
    /// Type for Rooms used in Backend.
    /// Since this ether can be a mock or a real room, it need to be named here.
    type Room: NCRoomInterface;
    /// Write all log files for this NC Instance to disk.
    /// # Errors
    /// Folder might not exists/be writable or not have space.
    fn write_to_log(&mut self) -> Result<(), std::io::Error>;
    /// Get a Room ref for a given Token.
    fn get_room(&self, token: &Token) -> &Self::Room;
    /// Get a list of tokens of rooms with unread messages.
    fn get_unread_rooms(&self) -> Vec<Token>;
    /// Get a list of tokens of favorite rooms.
    fn get_favorite_rooms(&self) -> Vec<Token>;
    /// Get a room token by its Displayname.
    fn get_room_by_displayname(&self, name: &str) -> Token;
    /// Get a list of direct messages rooms as token, displayname pairs.
    fn get_dm_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
    /// Get a list of group messages rooms as token, displayname pairs.
    fn get_group_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
    /// Get a list of all Room Token.
    fn get_room_keys(&self) -> Vec<&'_ Token>;
    /// Send a Message to the current selected room.
    async fn send_message(
        &mut self,
        message: String,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>>;
    /// Select a Room by a given Token as the current Room.
    async fn select_room(
        &mut self,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>>;
    /// Check with the Server for all Rooms if updates happened.
    /// ```force_update``` will force the currently stored Room data to be overwritten.
    async fn update_rooms(&mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>>;
    /// Mark the room identified by the Token as read.
    /// Does not need to be the current Room, but usually is.
    async fn mark_current_room_as_read(
        &self,
        token: &Token,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// Mark all rooms as read, goes over list of unread rooms.
    async fn mark_all_rooms_as_read(&self) -> Result<(), Box<dyn std::error::Error>>;
    /// Fetch a rooms full history.
    async fn fetch_room_history(&mut self, token: &Token) -> Result<(), Box<dyn Error>>;
    /// trigger for all threads to be killed.
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// NC Talk instance reprensation for all interactions with Server.
///
/// This struct stores all Rooms in a Hashmap and the API Wrapper.
#[derive(Debug, Default)]
pub struct NCTalk<Requester: NCRequestInterface + 'static + std::marker::Sync> {
    rooms: HashMap<Token, NCRoom>,
    chat_data_path: PathBuf,
    last_requested: i64,
    requester: Arc<Mutex<Requester>>,
}

impl<Requester: NCRequestInterface + 'static + std::marker::Send> NCTalk<Requester> {
    async fn parse_response(
        response: Vec<NCReqDataRoom>,
        raw_requester: Arc<Mutex<Requester>>,
        rooms: &mut HashMap<Token, NCRoom>,
        chat_log_path: PathBuf,
    ) {
        let v: Vec<JoinHandle<(String, Option<NCRoom>)>> = response
            .into_iter()
            .map(|child| {
                tokio::spawn(NCTalk::<Requester>::new_room(
                    child,
                    Arc::clone(&raw_requester),
                    chat_log_path.clone(),
                ))
            })
            .collect();
        log::debug!("Got {} initial threads", v.len());
        for jh in v {
            let (name, room_option) = jh.await.unwrap();
            if let Some(room) = room_option {
                rooms.insert(name, room);
            } else {
                log::warn!("Encountered a room that cannot be added {name}");
            }
        }
    }
    async fn parse_files(
        mut data: HashMap<Token, NCReqDataRoom>,
        requester: Arc<Mutex<Requester>>,
        chat_log_path: &Path,
        initial_message_ids: &mut HashMap<Token, &NCReqDataRoom>,
        rooms: &mut HashMap<Token, NCRoom>,
    ) -> Result<(), Box<dyn Error>> {
        let mut handles = HashMap::new();
        for (token, room) in &mut data {
            handles.insert(
                token.clone(),
                tokio::spawn(NCRoom::new::<Requester>(
                    room.clone(),
                    Arc::clone(&requester),
                    chat_log_path.to_path_buf(),
                )),
            );
        }
        log::debug!("Got {} initial threads", handles.capacity());
        for (token, room_future) in &mut handles {
            //we can safely unwrap here bc the json file on disk shall never be this broken.
            let mut json_room = room_future.await?.unwrap();
            if initial_message_ids.contains_key::<Token>(token) {
                let message_id = initial_message_ids.get(token).unwrap().lastMessage.id;
                json_room
                    .update_if_id_is_newer::<Requester>(
                        message_id,
                        Some((*initial_message_ids.get(token).unwrap()).clone()),
                        Arc::clone(&requester),
                    )
                    .await?;
                rooms.insert(token.clone(), json_room);
                initial_message_ids.remove(token);
            } else {
                log::warn!("Room was deleted upstream, failed to locate!");
                //TODO: remove old chat log!!
            }
        }
        Ok(())
    }

    async fn new_room(
        packaged_child: NCReqDataRoom,
        requester_box: Arc<Mutex<Requester>>,
        chat_log_path: PathBuf,
    ) -> (Token, Option<NCRoom>) {
        (
            packaged_child.token.clone(),
            NCRoom::new::<Requester>(packaged_child, requester_box, chat_log_path).await,
        )
    }
    /// Create a new NC Talk Backend instance.
    ///
    /// This will first try to read the chat history from the file system.
    /// Should this fail it will use the Requester to fetch data from Server.
    /// # Panics
    ///
    /// # Errors
    /// Initial fetching of the Rooms from the backend may fail.
    /// Selecting a current Room might fail.
    /// Reading data from a file might fail.
    pub async fn new(
        raw_requester: Requester,
        config: &Config,
    ) -> Result<NCTalk<Requester>, Box<dyn Error>> {
        let chat_log_path = config.get_server_data_dir();
        let mut tmp_path_buf = chat_log_path.clone();
        tmp_path_buf.push("Talk.json");
        let path = tmp_path_buf.as_path();
        log::debug!("Fetching initial Rooms List");

        let requester = Arc::new(Mutex::new(raw_requester));

        let resp = {
            requester
                .lock()
                .await
                .request_rooms_initial()
                .await
                .expect("Initial fetching of rooms on startup failed.")
        };
        let (response, last_requested) = resp
            .await
            .expect("Initial fetching of rooms failed.")
            .expect("No rooms found");
        log::debug!("Parsing initial Rooms List");

        let mut initial_message_ids: HashMap<Token, &NCReqDataRoom> = response
            .iter()
            .map(|room| (room.token.clone(), room))
            .collect::<HashMap<Token, &NCReqDataRoom>>();

        let mut rooms = HashMap::<Token, NCRoom>::new();

        log::debug!("Trying to read from disk.");

        if path.exists() {
            if let Ok(data) = serde_json::from_str::<HashMap<String, NCReqDataRoom>>(
                std::fs::read_to_string(path)?.as_str(),
            ) {
                let token_data = data
                    .iter()
                    .map(|(key, room_data)| (Token::from(key), room_data.clone()))
                    .collect();
                NCTalk::parse_files(
                    token_data,
                    Arc::clone(&requester),
                    chat_log_path.as_path(),
                    &mut initial_message_ids,
                    &mut rooms,
                )
                .await?;
                if !initial_message_ids.is_empty() {
                    let remaining_room_data = response
                        .iter()
                        .filter(|data| initial_message_ids.contains_key(&data.token))
                        .cloned()
                        .collect::<Vec<NCReqDataRoom>>();
                    NCTalk::<Requester>::parse_response(
                        remaining_room_data,
                        Arc::clone(&requester),
                        &mut rooms,
                        chat_log_path.clone(),
                    )
                    .await;
                    log::debug!(
                        "New Room adds, missing in logs {}",
                        initial_message_ids.len()
                    );
                }
                log::info!("Loaded Rooms from log files");
            } else {
                log::debug!("Failed to parse top level json, falling back to fetching");
                NCTalk::<Requester>::parse_response(
                    response,
                    requester.clone(),
                    &mut rooms,
                    chat_log_path.clone(),
                )
                .await;
            }
        } else {
            log::debug!("No Log files found in Path, fetching logs from server.");
            NCTalk::<Requester>::parse_response(
                response,
                requester.clone(),
                &mut rooms,
                chat_log_path.clone(),
            )
            .await;
        }

        let mut talk = NCTalk {
            rooms,
            chat_data_path: chat_log_path.clone(),
            last_requested,
            requester,
        };
        log::info!("Entering default room {}", config.data.ui.default_room);
        talk.select_room(&talk.get_room_by_displayname(&Token::from(&config.data.ui.default_room)))
            .await?;

        log::debug!("Found {} Rooms", talk.rooms.len());

        Ok(talk)
    }
}

#[async_trait]
impl<Requester: NCRequestInterface + 'static + std::marker::Sync> NCBackend for NCTalk<Requester> {
    type Room = NCRoom;
    fn write_to_log(&mut self) -> Result<(), std::io::Error> {
        use std::io::Write;

        let mut data = HashMap::<Token, NCReqDataRoom>::new();
        let mut tmp_path_buf = self.chat_data_path.clone();
        tmp_path_buf.push("Talk.json");
        let path = tmp_path_buf.as_path();
        for (key, room) in &mut self.rooms {
            data.insert(key.clone(), room.to_data());
            room.write_to_log()?;
        }
        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = match std::fs::File::create(path) {
            Err(why) => {
                log::error!(
                    "couldn't create top level log file {}: {}",
                    tmp_path_buf
                        .as_os_str()
                        .to_str()
                        .expect("Failed to convert"),
                    why
                );
                return Err(why);
            }
            Ok(file) => file,
        };

        if let Err(why) = file.write_all(serde_json::to_string(&data).unwrap().as_bytes()) {
            log::error!(
                "couldn't write top level log file to {}: {}",
                tmp_path_buf
                    .as_os_str()
                    .to_str()
                    .expect("Failed to convert"),
                why
            );
            Err(why)
        } else {
            log::debug!("Wrote Logs to files! {} ", tmp_path_buf.display());
            Ok(())
        }
    }

    fn get_unread_rooms(&self) -> Vec<Token> {
        self.rooms
            .values()
            .filter(|room| room.has_unread())
            .sorted_by(std::cmp::Ord::cmp)
            .map(NCRoomInterface::to_token)
            .collect::<Vec<Token>>()
    }

    fn get_favorite_rooms(&self) -> Vec<Token> {
        self.rooms
            .values()
            .filter(|room| room.is_favorite())
            .sorted()
            .map(NCRoomInterface::to_token)
            .collect()
    }

    fn get_room_by_displayname(&self, name: &str) -> Token {
        for room in self.rooms.values() {
            if room.to_string() == *name {
                return room.to_token();
            }
        }
        panic!("room doesnt exist {}", name);
    }

    fn get_dm_keys_display_name_mapping(&self) -> Vec<(Token, String)> {
        self.rooms
            .iter()
            .filter(|(_, room)| {
                [
                    NCRoomTypes::OneToOne,
                    NCRoomTypes::NoteToSelf,
                    NCRoomTypes::ChangeLog,
                ]
                .contains(room.get_room_type())
            })
            .map(|(key, _)| (key.clone(), self.rooms[key].to_string()))
            .sorted_by(|(token_a, _), (token_b, _)| self.rooms[token_a].cmp(&self.rooms[token_b]))
            .collect_vec()
    }

    fn get_group_keys_display_name_mapping(&self) -> Vec<(Token, String)> {
        let mut mapping: Vec<(Token, String)> = Vec::new();
        for (key, room) in &self.rooms {
            match room.get_room_type() {
                NCRoomTypes::Group | NCRoomTypes::Public => {
                    mapping.push((key.clone(), self.rooms[key].to_string()));
                }
                _ => {}
            }
        }
        mapping.sort_by(|(token_a, _), (token_b, _)| self.rooms[token_a].cmp(&self.rooms[token_b]));
        mapping
    }

    fn get_room_keys(&self) -> Vec<&Token> {
        self.rooms.keys().collect::<Vec<&Token>>()
    }

    async fn send_message(
        &mut self,
        message: String,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>> {
        self.rooms
            .get(token)
            .ok_or("Room not found when it should be there")?
            .send::<Requester>(message, Arc::clone(&self.requester))
            .await?;
        self.rooms
            .get_mut(token)
            .ok_or("Room not found when it should be there")?
            .update::<Requester>(None, Arc::clone(&self.requester))
            .await
    }

    async fn select_room(
        &mut self,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>> {
        log::debug!("selected room {}", token);
        self.rooms
            .get_mut(token)
            .ok_or_else(|| format!("Failed to get Room ref for room selection: {token}."))?
            .update::<Requester>(None, Arc::clone(&self.requester))
            .await
    }

    async fn update_rooms(&mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>> {
        let (response, timestamp) = if force_update {
            let resp = {
                self.requester
                    .lock()
                    .await
                    .request_rooms_update(self.last_requested)
                    .await
                    .expect("Initial fetching of rooms on startup failed.")
            };
            resp.await
                .expect("Initial fetching of rooms failed.")
                .expect("No rooms found")
        } else {
            let resp = {
                self.requester
                    .lock()
                    .await
                    .request_rooms_initial()
                    .await
                    .expect("Initial fetching of rooms on startup failed.")
            };
            resp.await
                .expect("Initial fetching of rooms failed.")
                .expect("No rooms found")
        };
        self.last_requested = timestamp;
        let mut new_room_token: Vec<String> = vec![];
        for room in response {
            if self.rooms.contains_key(&room.token) {
                let room_ref = self
                    .rooms
                    .get_mut(&room.token)
                    .ok_or("Failed to get Room ref for update.")?;
                if force_update {
                    room_ref
                        .update::<Requester>(Some(room), Arc::clone(&self.requester))
                        .await?;
                } else {
                    room_ref
                        .update_if_id_is_newer::<Requester>(
                            room.lastMessage.id,
                            Some(room),
                            Arc::clone(&self.requester),
                        )
                        .await?;
                }
            } else {
                new_room_token.push(room.displayName.clone());
                self.rooms.insert(
                    room.token.clone(),
                    NCRoom::new(room, self.requester.clone(), self.chat_data_path.clone())
                        .await
                        .expect("Could not Create Room."),
                );
            }
        }
        Ok(new_room_token)
    }

    async fn mark_current_room_as_read(
        &self,
        token: &Token,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.rooms[token]
            .mark_as_read(Arc::clone(&self.requester))
            .await
    }
    async fn mark_all_rooms_as_read(&self) -> Result<(), Box<dyn std::error::Error>> {
        for token in self.get_unread_rooms() {
            self.mark_current_room_as_read(&token).await?;
        }
        Ok(())
    }

    fn get_room(&self, token: &Token) -> &Self::Room {
        &self.rooms[token]
    }

    async fn fetch_room_history(&mut self, token: &Token) -> Result<(), Box<dyn Error>> {
        self.rooms
            .get_mut(token.as_str())
            .expect("Current Rooms seem to be missing.")
            .fill_history(Arc::clone(&self.requester))
            .await
    }
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.requester.lock().await.shutdown().await
    }
}

#[cfg(test)]
use crate::backend::nc_room::MockNCRoomInterface;
#[cfg(test)]
use mockall::{mock, predicate::*};

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub NCTalk{
    }
    #[async_trait]
    impl NCBackend for NCTalk{
        type Room = MockNCRoomInterface;
        fn write_to_log(&mut self) -> Result<(), std::io::Error>;
        fn get_room(&self, token: &Token) -> &<MockNCTalk as NCBackend>::Room;
        fn get_unread_rooms(&self) -> Vec<Token>;
        fn get_favorite_rooms(&self) -> Vec<Token>;
        fn get_room_by_displayname(&self, name: &str) -> Token;
        fn get_dm_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
        fn get_group_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
        fn get_room_keys<'a>(&'a self) -> Vec<&'a Token>;
        async fn send_message(& mut self, message: String, token: &Token) -> Result<Option<(String, usize)>, Box<dyn Error>>;
        async fn select_room(&mut self, token: &Token) -> Result<Option<(String, usize)>, Box<dyn Error>>;
        async fn update_rooms(& mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>>;
        async fn mark_current_room_as_read(&self, token: &Token) -> Result<(), Box<dyn std::error::Error>>;
        async fn mark_all_rooms_as_read(&self) -> Result<(), Box<dyn std::error::Error>>;
        async fn fetch_room_history(&mut self, token: &Token) -> Result<(), Box<dyn Error>>;
        async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
    }
}

#[cfg(test)]
static BUTZ: &str = "Butz";

#[cfg(test)]
impl std::fmt::Display for MockNCTalk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let self_name = BUTZ.to_string();
        write!(f, "{self_name}")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        backend::nc_request::{
            nc_requester::MockNCRequest, NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom,
        },
        config::init,
    };
    fn get_default_token() -> Token {
        Token::from("123")
    }

    fn get_default_room() -> NCReqDataRoom {
        NCReqDataRoom {
            displayName: "General".to_string(),
            token: get_default_token(),
            roomtype: 2, // Group Chat
            ..Default::default()
        }
    }

    fn get_default_message() -> NCReqDataMessage {
        NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 1,
            ..Default::default()
        }
    }

    fn prep_backend_creation(mock_requester: &mut MockNCRequest) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let (chat_tx, chat_rx) = tokio::sync::oneshot::channel();
        let (update_tx, update_rx) = tokio::sync::oneshot::channel();
        let (pat_tx, pat_rx) = tokio::sync::oneshot::channel();

        tx.send(Ok((vec![get_default_room()], 1)))
            .expect("Sending Failed.");

        chat_tx
            .send(Ok(vec![get_default_message()]))
            .expect("Sending Failed.");

        let update_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 2,
            ..Default::default()
        };
        update_tx
            .send(Ok(vec![update_message.clone()]))
            .expect("Sending Failed.");

        pat_tx
            .send(Ok(vec![NCReqDataParticipants::default()]))
            .expect("Sending Failed.");

        mock_requester
            .expect_request_rooms_initial()
            .once()
            .return_once(move || Ok(rx));
        mock_requester
            .expect_request_chat_initial()
            .with(eq(get_default_token()), eq(200))
            .return_once(move |_, _| Ok(chat_rx));

        mock_requester
            .expect_request_participants()
            .times(1)
            .return_once(move |_| Ok(pat_rx));
        mock_requester
            .expect_request_chat_update()
            .with(eq(get_default_token()), eq(200), eq(1))
            .return_once_st(move |_, _, _| Ok(update_rx));
    }

    #[tokio::test]
    async fn create_backend() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        let mut mock_requester = MockNCRequest::new();
        prep_backend_creation(&mut mock_requester);
        let backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert_eq!(backend.rooms.len(), 1);
    }
    #[tokio::test]
    async fn mark_room_as_read() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        let (chat_tx, chat_rx) = tokio::sync::oneshot::channel();
        chat_tx.send(Ok(())).expect("Sending Failed.");

        let mut mock_requester = MockNCRequest::new();
        prep_backend_creation(&mut mock_requester);
        mock_requester
            .expect_request_mark_chat_read()
            .with(eq(get_default_token()), eq(2))
            .return_once(move |_, _| Ok(chat_rx));

        let backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert!(backend
            .mark_current_room_as_read(&get_default_token())
            .await
            .is_ok());
    }
    #[tokio::test]
    async fn force_room_update() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        let mut mock_requester = MockNCRequest::new();

        prep_backend_creation(&mut mock_requester);

        let (tx2, rx2) = tokio::sync::oneshot::channel();
        let (chat_tx, chat_rx) = tokio::sync::oneshot::channel();

        let new_room = NCReqDataRoom {
            displayName: "General2".to_string(),
            token: Token::from("3456"),
            roomtype: 2, // Group Chat
            ..Default::default()
        };
        tx2.send(Ok((vec![get_default_room(), new_room], 2)))
            .expect("Sending Failed.");

        chat_tx
            .send(Ok(vec![get_default_message()]))
            .expect("Sending Failed.");

        mock_requester
            .expect_request_rooms_initial()
            .once()
            .return_once(move || Ok(rx2));
        mock_requester
            .expect_request_chat_initial()
            .with(eq(Token::from("3456")), eq(200))
            .return_once(move |_, _| Ok(chat_rx));

        let mut backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert_eq!(backend.rooms.len(), 1);

        assert!(backend.update_rooms(false).await.is_ok());
    }

    #[tokio::test]
    async fn room_handling() {
        let init = init("./test/").unwrap();
        let config = init;

        let mut mock_requester = MockNCRequest::new();

        let (tx2, rx2) = tokio::sync::oneshot::channel();
        let (pat2_tx, pat2_rx) = tokio::sync::oneshot::channel();
        let (send_tx, send_rx) = tokio::sync::oneshot::channel();
        let (chat_update_tx, chat_update_rx) = tokio::sync::oneshot::channel();

        let post_send_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 3,
            ..Default::default()
        };

        tx2.send(Ok((vec![get_default_room()], 1)))
            .expect("Sending Failed.");

        pat2_tx
            .send(Ok(vec![NCReqDataParticipants::default()]))
            .expect("Sending Failed.");

        send_tx
            .send(Ok(NCReqDataMessage::default()))
            .expect("Sending Failed");

        chat_update_tx
            .send(Ok(vec![post_send_message.clone()]))
            .expect("Failed to send");

        prep_backend_creation(&mut mock_requester);

        mock_requester
            .expect_request_rooms_initial()
            .once()
            .return_once(move || Ok(rx2));

        mock_requester
            .expect_request_send_message()
            .once()
            .withf(|message: &String, token: &Token| message == "Test" && *token == "123")
            .return_once(|_, _| Ok(send_rx));

        mock_requester
            .expect_request_chat_update()
            .once()
            .with(eq(get_default_token()), eq(200), eq(2))
            .return_once(move |_, _, _| Ok(chat_update_rx));

        mock_requester
            .expect_request_participants()
            .times(1)
            .return_once(move |_| Ok(pat2_rx));

        let mut backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");

        assert!(backend
            .send_message("Test".to_owned(), &Token::from("123"))
            .await
            .is_ok());

        assert!(backend.update_rooms(false).await.is_ok());

        assert_eq!(
            backend.get_room(&"123".into()).to_token(),
            Token::from("123")
        );
        assert_eq!(backend.get_unread_rooms().len(), 0);
        assert_eq!(
            backend.get_room_by_displayname("General"),
            Token::from("123")
        );
        assert_eq!(backend.get_dm_keys_display_name_mapping(), vec![]);
        assert_eq!(
            backend.get_group_keys_display_name_mapping(),
            vec![("123".into(), "General".to_string())]
        );
        assert_eq!(backend.get_room_keys(), vec![&Token::from("123")]);
    }

    #[tokio::test]
    async fn write_to_log() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();

        println!("Path is {}", config.get_data_dir().display());

        let mut mock_requester = MockNCRequest::new();
        prep_backend_creation(&mut mock_requester);

        let mut backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert_eq!(backend.rooms.len(), 1);

        backend.write_to_log().unwrap();
        dir.close().unwrap();
    }
}
