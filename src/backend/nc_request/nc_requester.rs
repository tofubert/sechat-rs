use tokio::{
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
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
    SendMessage(Token, String, oneshot::Sender<Option<NCReqDataMessage>>),
    FetchRoomsInitial(oneshot::Sender<Option<(Vec<NCReqDataRoom>, i64)>>),
    FetchRoomsUpdate(i64, oneshot::Sender<Option<(Vec<NCReqDataRoom>, i64)>>),
    FetchParticipants(Token, oneshot::Sender<Option<Vec<NCReqDataParticipants>>>),
    FetchChatInitial(Token, i32, oneshot::Sender<Option<Vec<NCReqDataMessage>>>),
    FetchChatUpdate(
        Token,
        i32,
        i32,
        oneshot::Sender<Option<Vec<NCReqDataMessage>>>,
    ),
    FetchAutocompleteUsers(String, oneshot::Sender<Option<Vec<NCReqDataUser>>>),
    MarkChatRead(Token, i32, oneshot::Sender<Option<()>>),
}

#[async_trait]
pub trait NCRequestInterface: Debug + Send + Send + Sync {
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> Result<tokio::sync::oneshot::Receiver<Option<NCReqDataMessage>>, Box<dyn Error>>;
    async fn request_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataUser>>>, Box<dyn Error>>;
    async fn request_participants(
        &self,
        token: &Token,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataParticipants>>>, Box<dyn Error>>;
    async fn request_rooms_initial(
        &self,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>>;
    async fn request_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>>;
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>>;
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>>;
    async fn request_mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<()>>, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct NCRequest {
    request_tx: Sender<ApiRequests>,
}

impl NCRequest {
    pub fn new(config: &Config) -> Self {
        let (tx, mut rx) = mpsc::channel::<ApiRequests>(50);

        let worker = NCRequestWorker::new(config).unwrap();
        log::warn!("Spawn Now");

        tokio::spawn(async move {
            loop {
                if let Some(req) = rx.recv().await {
                    match req {
                        ApiRequests::FetchChatInitial(token, maxMessage, response) => {
                            response
                                .send(Some(
                                    worker.fetch_chat_initial(&token, maxMessage).await.unwrap(),
                                ))
                                .expect("could not Send.");
                        }
                        ApiRequests::FetchChatUpdate(token, maxMessage, last_message, response) => {
                            response
                                .send(Some(
                                    worker
                                        .fetch_chat_update(&token, maxMessage, last_message)
                                        .await
                                        .unwrap(),
                                ))
                                .expect("could not Send.");
                        }
                        ApiRequests::FetchRoomsInitial(response) => {
                            response
                                .send(Some(worker.fetch_rooms_initial().await.unwrap()))
                                .expect("could not Send.");
                            log::warn!("Send Room Fetch Request");
                        }
                        ApiRequests::FetchRoomsUpdate(last_timestamp, response) => {
                            response
                                .send(Some(
                                    worker.fetch_rooms_update(last_timestamp).await.unwrap(),
                                ))
                                .expect("could not Send.");
                        }
                        ApiRequests::SendMessage(token, message, response) => {
                            response
                                .send(Some(worker.send_message(message, &token).await.unwrap()))
                                .expect("could not Send.");
                        }
                        ApiRequests::FetchAutocompleteUsers(name, response) => {
                            response
                                .send(Some(worker.fetch_autocomplete_users(&name).await.unwrap()))
                                .expect("could not Send.");
                        }
                        ApiRequests::FetchParticipants(token, response) => {
                            response
                                .send(Some(worker.fetch_participants(&token).await.unwrap()))
                                .expect("could not Send.");
                        }
                        ApiRequests::MarkChatRead(token, last_message, response) => {
                            worker.mark_chat_read(&token, last_message).await.unwrap();
                            response.send(Some(())).expect("could not Send.");
                        }
                        ApiRequests::None => {
                            log::warn!("Unknown Request");
                        }
                    }
                };
                sleep(Duration::from_millis(100)).await;
            }
        });
        log::warn!("Spawn Done");

        NCRequest { request_tx: tx }
    }
}

#[async_trait]
impl NCRequestInterface for NCRequest {
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> Result<tokio::sync::oneshot::Receiver<Option<NCReqDataMessage>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.request_tx
            .send(ApiRequests::SendMessage(token.clone(), message, tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }

    async fn request_rooms_initial(
        &self,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.request_tx
            .send(ApiRequests::FetchRoomsInitial(tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataUser>>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchAutocompleteUsers(name.to_string(), tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_participants(
        &self,
        token: &Token,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataParticipants>>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchParticipants(token.clone(), tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }

    async fn request_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchRoomsUpdate(last_timestamp, tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchChatInitial(token.clone(), maxMessage, tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchChatUpdate(
                token.clone(),
                maxMessage,
                last_message,
                tx,
            ))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<()>>, Box<dyn Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::MarkChatRead(
                token.to_string(),
                last_message,
                tx,
            ))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug, Default, Clone)]
    pub NCRequest {}     // Name of the mock struct, less the "Mock" prefix

    #[async_trait]
    impl NCRequestInterface for NCRequest {
      async fn request_send_message(
          &self,
          message: String,
          token: &Token,
      ) -> Result<tokio::sync::oneshot::Receiver<Option<NCReqDataMessage>>, Box<dyn Error>>;
      async fn request_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataUser>>>, Box<dyn Error>>;
    async fn request_participants(
        &self,
        token: &Token,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataParticipants>>>, Box<dyn Error>>;
    async fn request_rooms_initial(
        &self,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>>;
    async fn request_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<oneshot::Receiver<Option<(Vec<NCReqDataRoom>, i64)>>, Box<dyn Error>>;
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>>;
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<Vec<NCReqDataMessage>>>, Box<dyn Error>>;
    async fn request_mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<oneshot::Receiver<Option<()>>, Box<dyn Error>>;
    }
    impl Clone for NCRequest {   // specification of the trait to mock
        fn clone(&self) -> Self;
    }
}
