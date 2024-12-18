use crate::{
    backend::{
        nc_request::{NCReqDataRoom, NCRequestInterface},
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
};

use super::{
    nc_request::Token,
    nc_room::{NCRoom, NCRoomTypes},
};

#[async_trait]
pub trait NCBackend: Debug + Send + Default {
    type Room: NCRoomInterface;
    fn write_to_log(&mut self) -> Result<(), std::io::Error>;
    fn get_room(&self, token: &Token) -> &Self::Room;
    fn get_unread_rooms(&self) -> Vec<Token>;
    fn get_room_by_displayname(&self, name: &str) -> Token;
    fn get_dm_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
    fn get_group_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
    fn get_room_keys(&self) -> Vec<&'_ Token>;
    async fn send_message(
        &mut self,
        message: String,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>>;
    async fn select_room(
        &mut self,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>>;
    async fn update_rooms(&mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>>;
    async fn mark_current_room_as_read(
        &self,
        token: &Token,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Debug, Default)]
pub struct NCTalk<Requester: NCRequestInterface + 'static + std::marker::Sync> {
    rooms: HashMap<Token, NCRoom>,
    chat_data_path: PathBuf,
    last_requested: i64,
    requester: Requester,
}

impl<Requester: NCRequestInterface + 'static + std::marker::Send> NCTalk<Requester> {
    async fn parse_response(
        response: Vec<NCReqDataRoom>,
        requester: Requester,
        rooms: &mut HashMap<Token, NCRoom>,
        chat_log_path: PathBuf,
    ) {
        let v = response.into_iter().map(|child| {
            tokio::spawn(NCTalk::<Requester>::new_room(
                child,
                requester.clone(),
                chat_log_path.clone(),
            ))
        });
        for jh in v {
            let (name, room_option) = jh.await.unwrap();
            if let Some(room) = room_option {
                rooms.insert(name, room);
            } else {
                log::info!("Encountered a room that cannot be added {} ", name);
            }
        }
    }
    async fn parse_files(
        mut data: HashMap<Token, NCReqDataRoom>,
        requester: &Requester,
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
                    requester.clone(),
                    chat_log_path.to_path_buf(),
                )),
            );
        }
        for (token, room_future) in &mut handles {
            //we can safely unwrap here bc the json file on disk shall never be this broken.
            let mut json_room = room_future.await?.unwrap();
            if initial_message_ids.contains_key::<Token>(token) {
                let message_id = initial_message_ids.get(token).unwrap().lastMessage.id;
                json_room
                    .update_if_id_is_newer::<Requester>(
                        message_id,
                        Some((*initial_message_ids.get(token).unwrap()).clone()),
                        requester,
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
        requester_box: Requester,
        chat_log_path: PathBuf,
    ) -> (Token, Option<NCRoom>) {
        (
            packaged_child.token.clone(),
            NCRoom::new::<Requester>(packaged_child, requester_box, chat_log_path).await,
        )
    }
    pub async fn new(
        requester: Requester,
        config: &Config,
    ) -> Result<NCTalk<Requester>, Box<dyn Error>> {
        let chat_log_path = config.get_server_data_dir();
        let mut tmp_path_buf = chat_log_path.clone();
        tmp_path_buf.push("Talk.json");
        let path = tmp_path_buf.as_path();
        log::debug!("Fetching initial Rooms List");

        let (response, last_requested) = requester.fetch_rooms_initial().await?;
        log::debug!("Parsing initial Rooms List");

        let mut initial_message_ids: HashMap<Token, &NCReqDataRoom> = response
            .iter()
            .map(|room| (room.token.clone(), room))
            .collect::<HashMap<Token, &NCReqDataRoom>>();

        let mut rooms = HashMap::<Token, NCRoom>::new();

        log::debug!("Trying to read from disk.");

        if path.exists() {
            if let Ok(data) = serde_json::from_str::<HashMap<String, NCReqDataRoom>>(
                std::fs::read_to_string(path).unwrap().as_str(),
            ) {
                let token_data = data
                    .iter()
                    .map(|(key, room_data)| (Token::from(key), room_data.clone()))
                    .collect();
                NCTalk::parse_files(
                    token_data,
                    &requester,
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
                        requester.clone(),
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
                log::warn!(
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
            log::warn!(
                "couldn't write top level log file to {}: {}",
                tmp_path_buf
                    .as_os_str()
                    .to_str()
                    .expect("Failed to convert"),
                why
            );
            Err(why)
        } else {
            log::debug!("Wrote Logs to files! {:?} ", tmp_path_buf);
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
            .send::<Requester>(message, &self.requester)
            .await?;
        self.rooms
            .get_mut(token)
            .ok_or("Room not found when it should be there")?
            .update::<Requester>(None, &self.requester)
            .await
    }

    async fn select_room(
        &mut self,
        token: &Token,
    ) -> Result<Option<(String, usize)>, Box<dyn Error>> {
        log::debug!("key {}", token);
        self.rooms
            .get_mut(token)
            .ok_or_else(|| format!("Failed to get Room ref for room selection: {token}."))?
            .update::<Requester>(None, &self.requester)
            .await
    }

    async fn update_rooms(&mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>> {
        let (response, timestamp) = if force_update {
            self.requester
                .fetch_rooms_update(self.last_requested)
                .await?
        } else {
            self.requester.fetch_rooms_initial().await?
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
                        .update::<Requester>(Some(room), &self.requester)
                        .await?;
                } else {
                    room_ref
                        .update_if_id_is_newer::<Requester>(
                            room.lastMessage.id,
                            Some(room),
                            &self.requester,
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
        self.rooms[token].mark_as_read(&self.requester).await
    }
    fn get_room(&self, token: &Token) -> &Self::Room {
        &self.rooms[token]
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
        fn get_room_by_displayname(&self, name: &str) -> Token;
        fn get_dm_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
        fn get_group_keys_display_name_mapping(&self) -> Vec<(Token, String)>;
        fn get_room_keys<'a>(&'a self) -> Vec<&'a Token>;
        async fn send_message(& mut self, message: String, token: &Token) -> Result<Option<(String, usize)>, Box<dyn Error>>;
        async fn select_room(&mut self, token: &Token) -> Result<Option<(String, usize)>, Box<dyn Error>>;
        async fn update_rooms(& mut self, force_update: bool) -> Result<Vec<String>, Box<dyn Error>>;
        async fn mark_current_room_as_read(&self, token: &Token) -> Result<(), Box<dyn std::error::Error>>;
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

    use mockall::Sequence;

    use super::*;
    use crate::{
        backend::nc_request::{
            MockNCRequest, NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom,
        },
        config::init,
    };

    #[tokio::test]
    async fn create_backend() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();
        let mut mock_requester = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_file = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_fetch = crate::backend::nc_request::MockNCRequest::new();
        let mock_requester_room = crate::backend::nc_request::MockNCRequest::new();

        let default_room = NCReqDataRoom {
            displayName: "General".to_string(),
            roomtype: 2, // Group Chat
            ..Default::default()
        };

        let default_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 1,
            ..Default::default()
        };
        let update_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 2,
            ..Default::default()
        };

        mock_requester
            .expect_fetch_rooms_initial()
            .once()
            .returning_st(move || Ok((vec![default_room.clone()], 0)));
        mock_requester_fetch
            .expect_fetch_chat_initial()
            .return_once_st(move |_, _| Ok(vec![default_message.clone()]));
        mock_requester_fetch
            .expect_fetch_participants()
            .times(1)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));

        mock_requester
            .expect_fetch_chat_update()
            .return_once_st(move |_, _, _| Ok(vec![update_message.clone()]));

        mock_requester
            .expect_fetch_participants()
            .times(1)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));

        mock_requester_file
            .expect_clone()
            .return_once_st(|| mock_requester_fetch);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_file);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_room);

        let backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert_eq!(backend.rooms.len(), 1);
    }

    #[tokio::test]
    async fn room_handling() {
        let init = init("./test/").unwrap();
        let config = init;

        let mut mock_requester = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_file = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_fetch = crate::backend::nc_request::MockNCRequest::new();
        let mock_requester_room = crate::backend::nc_request::MockNCRequest::new();

        let default_token = Token::from("123");

        let default_room = NCReqDataRoom {
            displayName: "General".to_string(),
            roomtype: 2, // Group Chat
            token: default_token.clone(),
            ..Default::default()
        };

        let default_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 1,
            ..Default::default()
        };
        let update_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 2,
            ..Default::default()
        };

        let post_send_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 3,
            ..Default::default()
        };

        let mut seq = Sequence::new();

        mock_requester
            .expect_fetch_rooms_initial()
            .times(2)
            .returning_st(move || Ok((vec![default_room.clone()], 0)));
        mock_requester_fetch
            .expect_fetch_chat_initial()
            .return_once_st(move |_, _| Ok(vec![default_message.clone()]));
        mock_requester_fetch
            .expect_fetch_participants()
            .times(1)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));
        mock_requester
            .expect_fetch_participants()
            .times(2)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));
        mock_requester
            .expect_fetch_chat_update()
            .with(eq(default_token.clone()), eq(200), eq(1))
            .once()
            .in_sequence(&mut seq)
            .return_once_st(move |_, _, _| Ok(vec![update_message.clone()]));
        mock_requester
            .expect_send_message()
            .once()
            .in_sequence(&mut seq)
            .withf(|message: &String, token: &Token| message == "Test" && *token == "123")
            .return_once_st(|_, _| Ok(NCReqDataMessage::default()));

        mock_requester
            .expect_fetch_chat_update()
            .once()
            .with(eq(Token::from("123")), eq(200), eq(2))
            .in_sequence(&mut seq)
            .return_once_st(move |_, _, _| Ok(vec![post_send_message.clone()]));

        mock_requester_file
            .expect_clone()
            .return_once_st(|| mock_requester_fetch);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_file);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_room);

        let backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");

        check_results(backend).await;
    }

    async fn check_results(mut backend: NCTalk<MockNCRequest>) {
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

        let mut mock_requester = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_file = crate::backend::nc_request::MockNCRequest::new();
        let mut mock_requester_fetch = crate::backend::nc_request::MockNCRequest::new();
        let mock_requester_room = crate::backend::nc_request::MockNCRequest::new();

        let default_room = NCReqDataRoom {
            displayName: "General".to_string(),
            token: "a123".into(),
            roomtype: 2, // Group Chat
            ..Default::default()
        };

        let default_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 1,
            ..Default::default()
        };
        let update_message = NCReqDataMessage {
            messageType: "comment".to_string(),
            id: 2,
            ..Default::default()
        };

        mock_requester
            .expect_fetch_rooms_initial()
            .once()
            .returning_st(move || Ok((vec![default_room.clone()], 0)));
        mock_requester_fetch
            .expect_fetch_chat_initial()
            .return_once_st(move |_, _| Ok(vec![default_message.clone()]));
        mock_requester_fetch
            .expect_fetch_participants()
            .times(1)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));
        mock_requester
            .expect_fetch_participants()
            .times(1)
            .returning_st(move |_| Ok(vec![NCReqDataParticipants::default()]));
        mock_requester
            .expect_fetch_chat_update()
            .return_once_st(move |_, _, _| Ok(vec![update_message.clone()]));

        mock_requester_file
            .expect_clone()
            .return_once_st(|| mock_requester_fetch);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_file);

        mock_requester
            .expect_clone()
            .return_once_st(|| mock_requester_room);

        let mut backend = NCTalk::new(mock_requester, &config)
            .await
            .expect("Failed to create Backend");
        assert_eq!(backend.rooms.len(), 1);

        backend.write_to_log().unwrap();
        dir.close().unwrap();
    }
}
