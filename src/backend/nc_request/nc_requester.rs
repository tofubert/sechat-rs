use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::{sleep, Duration},
};

use crate::config::Config;
use async_trait::async_trait;

use std::error::Error;
use std::fmt::Debug;

#[cfg(test)]
use mockall::{mock, predicate::*};

use super::{
    nc_req_worker::NCRequestWorker, NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom,
    NCReqDataUser, Token,
};

#[derive(Default)]
pub enum ApiRequests {
    #[default]
    None,
    SendMessage(Token, String),
    FetchRoomsInitial,
    FetchRoomsUpdate(i64),
    FetchParticipants(Token),
    FetchChatInitial(Token, i32),
    FetchChatUpdate(Token, i32, i32),
    FetchAutocompleteUsers(String),
    MarkChatRead(Token, i32),
}

#[async_trait]
pub trait NCRequestInterface: Debug + Send + Send + Sync {
    async fn fetch_send_message(&mut self) -> Option<NCReqDataMessage>;
    async fn fetch_autocomplete_users(&mut self) -> Option<Vec<NCReqDataUser>>;
    async fn fetch_participants(&mut self) -> Option<Vec<NCReqDataParticipants>>;
    async fn fetch_rooms_initial(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)>;
    async fn fetch_rooms_update(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)>;
    async fn fetch_chat_initial(&mut self) -> Option<Vec<NCReqDataMessage>>;
    async fn fetch_chat_update(&mut self) -> Option<Vec<NCReqDataMessage>>;
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> Result<(), Box<dyn Error>>;
    async fn request_autocomplete_users(&self, name: &str) -> Result<(), Box<dyn Error>>;
    async fn request_participants(&self, token: &Token) -> Result<(), Box<dyn Error>>;
    async fn request_rooms_initial(&self) -> Result<(), Box<dyn Error>>;
    async fn request_rooms_update(&self, last_timestamp: i64) -> Result<(), Box<dyn Error>>;
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> Result<(), Box<dyn Error>>;
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<(), Box<dyn Error>>;
    async fn request_mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]
pub struct NCRequest {
    request_tx: Sender<ApiRequests>,
    rx_fetch_room_initial: Receiver<(Vec<NCReqDataRoom>, i64)>,
    rx_fetch_room_update: Receiver<(Vec<NCReqDataRoom>, i64)>,
    rx_fetch_chat_initial: Receiver<Vec<NCReqDataMessage>>,
    rx_fetch_chat_update: Receiver<Vec<NCReqDataMessage>>,
    rx_fetch_send_message: Receiver<NCReqDataMessage>,
    rx_fetch_participants: Receiver<Vec<NCReqDataParticipants>>,
    rx_fetch_autocomplete_users: Receiver<Vec<NCReqDataUser>>,
}

impl NCRequest {
    pub fn new(config: &Config) -> Result<NCRequest, Box<dyn Error>> {
        let (tx, mut rx) = mpsc::channel::<ApiRequests>(50);
        let (tx_fetch_room_initial, rx_fetch_room_initial) =
            mpsc::channel::<(Vec<NCReqDataRoom>, i64)>(10);
        let (tx_fetch_room_update, rx_fetch_room_update) =
            mpsc::channel::<(Vec<NCReqDataRoom>, i64)>(10);
        let (tx_fetch_chat_initial, rx_fetch_chat_initial) =
            mpsc::channel::<Vec<NCReqDataMessage>>(10);
        let (tx_fetch_chat_update, rx_fetch_chat_update) =
            mpsc::channel::<Vec<NCReqDataMessage>>(10);
        let (tx_fetch_send_message, rx_fetch_send_message) = mpsc::channel::<NCReqDataMessage>(10);
        let (tx_fetch_participants, rx_fetch_participants) =
            mpsc::channel::<Vec<NCReqDataParticipants>>(10);
        let (tx_fetch_autocomplete_users, rx_fetch_autocomplete_users) =
            mpsc::channel::<Vec<NCReqDataUser>>(10);

        let worker = NCRequestWorker::new(config).unwrap();
        log::warn!("Spawn Now");

        tokio::spawn(async move {
            loop {
                if let Some(req) = rx.recv().await {
                    match req {
                        ApiRequests::FetchChatInitial(token, maxMessage) => {
                            tx_fetch_chat_initial
                                .send(worker.fetch_chat_initial(&token, maxMessage).await.unwrap())
                                .await;
                        }
                        ApiRequests::FetchChatUpdate(token, maxMessage, last_message) => {
                            tx_fetch_chat_update
                                .send(
                                    worker
                                        .fetch_chat_update(&token, maxMessage, last_message)
                                        .await
                                        .unwrap(),
                                )
                                .await;
                        }
                        ApiRequests::FetchRoomsInitial => {
                            tx_fetch_room_initial
                                .send(worker.fetch_rooms_initial().await.unwrap())
                                .await;
                            log::warn!("Send Room Fetch Request");
                        }
                        _ => {
                            log::warn!("Unknown Request");
                        }
                    }
                };
                sleep(Duration::from_millis(100)).await;
            }
        });
        log::warn!("Spawn Done");

        Ok(NCRequest {
            request_tx: tx,
            rx_fetch_room_initial,
            rx_fetch_room_update,
            rx_fetch_chat_initial,
            rx_fetch_chat_update,
            rx_fetch_send_message,
            rx_fetch_participants,
            rx_fetch_autocomplete_users,
        })
    }
}

#[async_trait]
impl NCRequestInterface for NCRequest {
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::SendMessage(token.clone(), message))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }

    async fn request_rooms_initial(&self) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchRoomsInitial)
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_autocomplete_users(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchAutocompleteUsers(name.to_string()))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_participants(&self, token: &Token) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchParticipants(token.clone()))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_rooms_update(&self, last_timestamp: i64) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchRoomsUpdate(last_timestamp))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchChatInitial(token.clone(), maxMessage))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::FetchChatUpdate(
                token.clone(),
                maxMessage,
                last_message,
            ))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }
    async fn request_mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<(), Box<dyn Error>> {
        self.request_tx
            .send(ApiRequests::MarkChatRead(token.to_string(), last_message))
            .await
            .expect("Queing request for sending of message failed.");
        Ok(())
    }

    async fn fetch_send_message(&mut self) -> Option<NCReqDataMessage> {
        self.rx_fetch_send_message.recv().await
    }
    async fn fetch_rooms_initial(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)> {
        self.rx_fetch_room_initial.recv().await
    }

    async fn fetch_autocomplete_users(&mut self) -> Option<Vec<NCReqDataUser>> {
        self.rx_fetch_autocomplete_users.recv().await
    }
    async fn fetch_participants(&mut self) -> Option<Vec<NCReqDataParticipants>> {
        self.rx_fetch_participants.recv().await
    }
    async fn fetch_rooms_update(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)> {
        self.rx_fetch_room_update.recv().await
    }
    async fn fetch_chat_initial(&mut self) -> Option<Vec<NCReqDataMessage>> {
        self.rx_fetch_chat_initial.recv().await
    }
    async fn fetch_chat_update(&mut self) -> Option<Vec<NCReqDataMessage>> {
        self.rx_fetch_chat_update.recv().await
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug, Default, Clone)]
    pub NCRequest {}     // Name of the mock struct, less the "Mock" prefix

    #[async_trait]
    impl NCRequestInterface for NCRequest {
      async fn fetch_send_message(&mut self) -> Option<NCReqDataMessage>;
      async fn fetch_autocomplete_users(&mut self) -> Option<Vec<NCReqDataUser>>;
      async fn fetch_participants(&mut self) -> Option<Vec<NCReqDataParticipants>>;
      async fn fetch_rooms_initial(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)>;
      async fn fetch_rooms_update(&mut self) -> Option<(Vec<NCReqDataRoom>, i64)>;
      async fn fetch_chat_initial(&mut self) -> Option<Vec<NCReqDataMessage>>;
      async fn fetch_chat_update(&mut self) -> Option<Vec<NCReqDataMessage>>;
      async fn request_send_message(
          &self,
          message: String,
          token: &Token,
      ) -> Result<(), Box<dyn Error>>;
      async fn request_autocomplete_users(&self, name: &str) -> Result<(), Box<dyn Error>>;
      async fn request_participants(&self, token: &Token) -> Result<(), Box<dyn Error>>;
      async fn request_rooms_initial(&self) -> Result<(), Box<dyn Error>>;
      async fn request_rooms_update(&self, last_timestamp: i64) -> Result<(), Box<dyn Error>>;
      async fn request_chat_initial(
          &self,
          token: &Token,
          maxMessage: i32,
      ) -> Result<(), Box<dyn Error>>;
      async fn request_chat_update(
          &self,
          token: &Token,
          maxMessage: i32,
          last_message: i32,
      ) -> Result<(), Box<dyn Error>>;
      async fn request_mark_chat_read(
          &self,
          token: &str,
          last_message: i32,
      ) -> Result<(), Box<dyn Error>>;
    }
    impl Clone for NCRequest {   // specification of the trait to mock
        fn clone(&self) -> Self;
    }
}
