use crate::{
    backend::{
        nc_notify::NCNotify,
        nc_request::{NCReqDataRoom, NCRequest},
        nc_room::NCRoomInterface,
    },
    config::{self},
};
use async_trait::async_trait;
use itertools::Itertools;
use std::{collections::HashMap, error::Error, fmt::Debug, path::PathBuf};

use super::nc_room::{NCRoom, NCRoomTypes};

#[async_trait]
pub trait NCBackend: Debug + Send + Default {
    type Room: NCRoomInterface;
    fn write_to_log(&mut self) -> Result<(), std::io::Error>;
    fn get_current_room_token(&self) -> &str;
    fn get_room(&self, token: &str) -> &Self::Room;
    fn get_current_room(&self) -> &Self::Room;
    fn get_unread_rooms(&self) -> Vec<String>;
    fn get_room_by_displayname(&self, name: &str) -> &str;
    fn get_dm_keys_display_name_mapping(&self) -> Vec<(String, String)>;
    fn get_group_keys_display_name_mapping(&self) -> Vec<(String, String)>;
    fn get_room_keys(&self) -> Vec<&'_ String>;
    async fn send_message(&mut self, message: String) -> Result<(), Box<dyn Error>>;
    async fn select_room(&mut self, token: String) -> Result<(), Box<dyn Error>>;
    async fn update_rooms(&mut self, force_update: bool) -> Result<(), Box<dyn Error>>;
    fn add_room(&mut self, room_option: Option<Self::Room>);
}

#[derive(Debug, Default)]
pub struct NCTalk {
    rooms: HashMap<String, NCRoom>,
    chat_data_path: PathBuf,
    last_requested: i64,
    requester: NCRequest,
    notifier: NCNotify,
    pub current_room_token: String,
}

impl NCTalk {
    async fn parse_response(
        response: Vec<NCReqDataRoom>,
        requester: NCRequest,
        notifier: NCNotify,
        rooms: &mut HashMap<String, NCRoom>,
        chat_log_path: PathBuf,
    ) {
        // for res in response {
        //     let room_tuple = NCTalk::<Room>::new_room(
        //         res,
        //         requester.clone(),
        //         notifier.clone(),
        //         chat_log_path.clone(),
        //     );
        //     let (name, room_option) = room_tuple.await;
        //     if let Some(room) = room_option {
        //         rooms.insert(name, room);
        //     } else {
        //         log::info!("Encountered a room that cannot be added {} ", name);
        //     }
        // }

        let v = response.into_iter().map(|child| {
            tokio::spawn(NCTalk::new_room(
                child,
                requester.clone(),
                notifier.clone(),
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

    async fn new_room(
        packaged_child: NCReqDataRoom,
        requester_box: NCRequest,
        notifier: NCNotify,
        chat_log_path: PathBuf,
    ) -> (String, Option<NCRoom>) {
        (
            packaged_child.token.clone(),
            NCRoom::new(packaged_child, requester_box, notifier, chat_log_path).await,
        )
    }
    pub async fn new(requester: NCRequest) -> Result<NCTalk, Box<dyn Error>> {
        let notify = NCNotify::new();

        let chat_log_path = config::get().get_server_data_dir();
        let mut tmp_path_buf = chat_log_path.clone();
        tmp_path_buf.push("Talk.json");
        let path = tmp_path_buf.as_path();

        let (response, last_requested) = requester.fetch_rooms_initial().await?;

        let mut initial_message_ids: HashMap<String, &NCReqDataRoom> = response
            .iter()
            .map(|room| (room.token.clone(), room))
            .collect::<HashMap<String, &NCReqDataRoom>>();

        let mut rooms = HashMap::<String, NCRoom>::new();

        if path.exists() {
            if let Ok(mut data) = serde_json::from_str::<HashMap<String, NCReqDataRoom>>(
                std::fs::read_to_string(path).unwrap().as_str(),
            ) {
                let mut handles = HashMap::new();
                for (token, room) in &mut data {
                    handles.insert(
                        token.clone(),
                        tokio::spawn(NCRoom::new(
                            room.clone(),
                            requester.clone(),
                            notify.clone(),
                            chat_log_path.clone(),
                        )),
                    );
                }
                for (token, room_future) in &mut handles {
                    //we can safely unwrap here bc the json file on disk shall never be this broken.
                    let mut json_room = room_future.await?.unwrap();
                    if initial_message_ids.contains_key::<String>(token) {
                        let message_id = initial_message_ids.get(token).unwrap().lastMessage.id;
                        json_room
                            .update_if_id_is_newer(
                                message_id,
                                Some((*initial_message_ids.get(token).unwrap()).clone()),
                            )
                            .await?;
                        rooms.insert(token.clone(), json_room);
                        initial_message_ids.remove(token);
                    } else {
                        log::warn!("Room was deleted upstream, failed to locate!");
                        //TODO: remove old chat log!!
                    }
                }
                if !initial_message_ids.is_empty() {
                    let remaining_room_data = response
                        .iter()
                        .filter(|data| initial_message_ids.contains_key(&data.token))
                        .cloned()
                        .collect::<Vec<NCReqDataRoom>>();
                    NCTalk::parse_response(
                        remaining_room_data,
                        requester.clone(),
                        notify.clone(),
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
                NCTalk::parse_response(
                    response,
                    requester.clone(),
                    notify.clone(),
                    &mut rooms,
                    chat_log_path.clone(),
                )
                .await;
            }
        } else {
            log::debug!("No Log files found in Path, fetching logs from server.");
            NCTalk::parse_response(
                response,
                requester.clone(),
                notify.clone(),
                &mut rooms,
                chat_log_path.clone(),
            )
            .await;
        }

        let mut talk = NCTalk {
            rooms,
            chat_data_path: chat_log_path.clone(),
            last_requested,
            current_room_token: String::new(),
            requester,
            notifier: notify,
        };
        log::info!(
            "Entering default room {}",
            &config::get().data.ui.default_room
        );
        talk.select_room(
            talk.get_room_by_displayname(&config::get().data.ui.default_room)
                .to_string(),
        )
        .await?;

        log::debug!("Found {} Rooms", talk.rooms.len());

        Ok(talk)
    }
}

#[async_trait]
impl NCBackend for NCTalk {
    type Room = NCRoom;
    fn write_to_log(&mut self) -> Result<(), std::io::Error> {
        use std::io::Write;

        let mut data = HashMap::<String, NCReqDataRoom>::new();
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

    fn get_current_room_token(&self) -> &str {
        self.current_room_token.as_str()
    }

    fn get_current_room(&self) -> &Self::Room {
        &self.rooms[&self.current_room_token]
    }

    fn get_room(&self, token: &str) -> &Self::Room {
        &self.rooms[token]
    }

    fn get_unread_rooms(&self) -> Vec<String> {
        self.rooms
            .values()
            .filter(|room| room.has_unread() && self.current_room_token != room.to_token())
            .sorted_by(std::cmp::Ord::cmp)
            .map(NCRoomInterface::to_token)
            .collect::<Vec<String>>()
    }

    fn get_room_by_displayname(&self, name: &str) -> &str {
        for room in self.rooms.values() {
            if room.to_string() == *name {
                return self.rooms[&room.to_token()].as_str();
            }
        }
        panic!("room doesnt exist {}", name);
    }

    fn get_dm_keys_display_name_mapping(&self) -> Vec<(String, String)> {
        self.rooms
            .iter()
            .filter(|(_, room)| {
                [
                    NCRoomTypes::OneToOne,
                    NCRoomTypes::NoteToSelf,
                    NCRoomTypes::ChangeLog,
                ]
                .contains(&room.get_room_type())
            })
            .map(|(key, _)| (key.clone(), self.rooms[key].to_string()))
            .sorted_by(|(token_a, _), (token_b, _)| self.rooms[token_a].cmp(&self.rooms[token_b]))
            .collect_vec()
    }

    fn get_group_keys_display_name_mapping(&self) -> Vec<(String, String)> {
        let mut mapping: Vec<(String, String)> = Vec::new();
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

    fn get_room_keys(&self) -> Vec<&String> {
        self.rooms.keys().collect::<Vec<&String>>()
    }

    async fn send_message(&mut self, message: String) -> Result<(), Box<dyn Error>> {
        self.rooms
            .get(&self.current_room_token)
            .ok_or("Room not found when it should be there")?
            .send(message)
            .await?;
        self.rooms
            .get_mut(&self.current_room_token)
            .ok_or("Room not found when it should be there")?
            .update(None)
            .await
    }

    async fn select_room(&mut self, token: String) -> Result<(), Box<dyn Error>> {
        self.current_room_token.clone_from(&token);
        self.rooms
            .get_mut(&self.current_room_token)
            .ok_or("Failed to get Room ref")?
            .update(None)
            .await
    }

    async fn update_rooms(&mut self, force_update: bool) -> Result<(), Box<dyn Error>> {
        let (response, timestamp) = if force_update {
            self.requester
                .fetch_rooms_update(self.last_requested)
                .await?
        } else {
            self.requester.fetch_rooms_initial().await?
        };
        self.last_requested = timestamp;
        for room in response {
            if self.rooms.contains_key(room.token.as_str()) {
                let room_ref = self
                    .rooms
                    .get_mut(room.token.as_str())
                    .ok_or("Failed to get Room ref.")?;
                if force_update {
                    room_ref.update(Some(room)).await?;
                } else {
                    room_ref
                        .update_if_id_is_newer(room.lastMessage.id, Some(room))
                        .await?;
                }
            } else {
                self.notifier.new_room(&room.displayName)?;
                self.add_room(
                    NCRoom::new(
                        room,
                        self.requester.clone(),
                        self.notifier.clone(),
                        self.chat_data_path.clone(),
                    )
                    .await,
                );
            }
        }
        Ok(())
    }

    fn add_room(&mut self, room_option: Option<Self::Room>) {
        if let Some(room) = room_option {
            self.rooms.insert(room.to_token(), room);
        }
    }
}

#[cfg(test)]
use crate::backend::nc_room::MockNCRoomInterface;
#[cfg(test)]
use mockall::{mock, predicate::*};

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub NCTalk{}
    #[async_trait]
    impl NCBackend for NCTalk{
        type Room = MockNCRoomInterface;
        fn write_to_log(&mut self) -> Result<(), std::io::Error>;
        fn get_current_room_token(&self) -> &str;
        fn get_room(&self, token: &str) -> &<MockNCTalk as NCBackend>::Room;
        fn get_current_room(&self) -> &<MockNCTalk as NCBackend>::Room;
        fn get_unread_rooms(&self) -> Vec<String>;
        fn get_room_by_displayname(&self, name: &str) -> &str;
        fn get_dm_keys_display_name_mapping(&self) -> Vec<(String, String)>;
        fn get_group_keys_display_name_mapping(&self) -> Vec<(String, String)>;
        fn get_room_keys<'a>(&'a self) -> Vec<&'a String>;
        async fn send_message(& mut self, message: String) -> Result<(), Box<dyn Error>>;
        async fn select_room(&mut self, token: String) -> Result<(), Box<dyn Error>>;
        async fn update_rooms(& mut self, force_update: bool) -> Result<(), Box<dyn Error>>;
        fn add_room(&mut self, room_option: Option<<MockNCTalk as NCBackend>::Room>);
    }
}
#[cfg(test)]
impl std::fmt::Display for MockNCTalk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    // use crate::backend::nc_room::MockNCRoomInterface;

    // use super::*;

    // #[derive(Debug, Default)]
    // pub struct MockBackend {
    //     pub current_token: String,
    //     pub room: HashMap<String, Box<dyn NCRoomInterface>>,
    // }

    // impl<'a, Room> NCBackend<'a, Room> for MockBackend<'a, Room> {
    //     fn get_room(&self, token: &str) -> &Room {}
    //     fn get_current_room(&self, token: &str) -> &Room {}
    //     fn write_to_log(&mut self) -> Result<(), std::io::Error> {
    //         Ok(())
    //     }
    //     fn get_current_room_token(&self) -> &str {}
    //     fn get_unread_rooms(&self) -> Vec<String> {
    //         vec![]
    //     }
    //     fn get_room_by_displayname(&self, name: &str) -> &str {}
    //     fn get_dm_keys_display_name_mapping(&self) -> Vec<(String, String)> {
    //         vec![]
    //     }
    //     fn get_group_keys_display_name_mapping(&self) -> Vec<(String, String)> {
    //         vec![]
    //     }
    //     fn get_room_keys(&self) -> Vec<&String> {
    //         self.rooms.keys().collect::<Vec<&String>>()
    //     }
    // }
    // #[async_trait]
    // impl NCBackendAsync for MockBackend {
    //     async fn send_message(&mut self, _message: String) -> Result<(), Box<dyn Error>> {
    //         Ok(())
    //     }
    //     async fn select_room(&mut self, _token: String) -> Result<(), Box<dyn Error>> {
    //         Ok(())
    //     }
    //     async fn update_rooms(&mut self, _force_update: bool) -> Result<(), Box<dyn Error>> {
    //         Ok(())
    //     }
    // }
}
