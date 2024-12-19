use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};

use crate::config::Config;
use async_trait::async_trait;

use std::fmt::Debug;
use std::{error::Error, fmt};

#[cfg(test)]
use mockall::{mock, predicate::*};

use super::{
    nc_req_worker::NCRequestWorker, NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom,
    NCReqDataUser, Token,
};

#[derive(Default, Debug)]
pub enum ApiRequests {
    #[default]
    None,
    SendMessage(
        Token,
        String,
        oneshot::Sender<Result<NCReqDataMessage, Box<dyn Error>>>,
    ),
    FetchRoomsInitial(oneshot::Sender<Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>>),
    FetchRoomsUpdate(
        i64,
        oneshot::Sender<Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>>,
    ),
    FetchParticipants(
        Token,
        oneshot::Sender<Result<Vec<NCReqDataParticipants>, Box<dyn Error>>>,
    ),
    FetchChatInitial(
        Token,
        i32,
        oneshot::Sender<Result<Vec<NCReqDataMessage>, Box<dyn Error>>>,
    ),
    FetchChatUpdate(
        Token,
        i32,
        i32,
        oneshot::Sender<Result<Vec<NCReqDataMessage>, Box<dyn Error>>>,
    ),
    FetchAutocompleteUsers(
        String,
        oneshot::Sender<Result<Vec<NCReqDataUser>, Box<dyn Error>>>,
    ),
    MarkChatRead(Token, i32, oneshot::Sender<Result<(), Box<dyn Error>>>),
}

impl fmt::Display for ApiRequests {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiRequests::None => write!(f, "Invalid"),
            ApiRequests::SendMessage(token, _, _) => write!(f, "SendMessage {token}"),
            ApiRequests::FetchRoomsInitial(_) => write!(f, "FetchRoomsInitial"),
            ApiRequests::FetchRoomsUpdate(last_timestamp, _) => {
                write!(f, "FetchRoomsUpdate {last_timestamp}")
            }
            ApiRequests::FetchParticipants(token, _) => write!(f, "FetchParticipants {token}"),
            ApiRequests::FetchChatInitial(token, maxMessage, _) => {
                write!(f, "FetchChatInitial {token} {maxMessage}")
            }
            ApiRequests::FetchChatUpdate(token, maxMessage, last_message, _) => {
                write!(f, "FetchChatUpdate {token} {maxMessage} {last_message}")
            }
            ApiRequests::FetchAutocompleteUsers(name, _) => {
                write!(f, "FetchAutocompleteUsers {name}")
            }
            ApiRequests::MarkChatRead(token, i32, _) => write!(f, "MarkChatRead {token}"),
        }
    }
}

type ApiResult<T> =
    Result<tokio::sync::oneshot::Receiver<Result<T, Box<dyn Error>>>, Box<dyn Error>>;

#[async_trait]
pub trait NCRequestInterface: Debug + Send + Send + Sync {
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> ApiResult<NCReqDataMessage>;
    async fn request_autocomplete_users(&self, name: &str) -> ApiResult<Vec<NCReqDataUser>>;
    async fn request_participants(&self, token: &Token) -> ApiResult<Vec<NCReqDataParticipants>>;
    async fn request_rooms_initial(&self) -> ApiResult<(Vec<NCReqDataRoom>, i64)>;
    async fn request_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> ApiResult<(Vec<NCReqDataRoom>, i64)>;
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> ApiResult<Vec<NCReqDataMessage>>;
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> ApiResult<Vec<NCReqDataMessage>>;
    async fn request_mark_chat_read(&self, token: &str, last_message: i32) -> ApiResult<()>;
}

#[derive(Debug)]
pub struct NCRequest {
    request_tx: Sender<ApiRequests>,
}

impl NCRequest {
    async fn handle_req(worker: &NCRequestWorker, req: ApiRequests) {
        log::debug!("got a new API Request {}", req);
        match req {
            ApiRequests::FetchChatInitial(token, maxMessage, response) => {
                response
                    .send(Ok(worker
                        .fetch_chat_initial(&token, maxMessage)
                        .await
                        .unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::FetchChatUpdate(token, maxMessage, last_message, response) => {
                response
                    .send(Ok(worker
                        .fetch_chat_update(&token, maxMessage, last_message)
                        .await
                        .unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::FetchRoomsInitial(response) => {
                response
                    .send(Ok(worker.fetch_rooms_initial().await.unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::FetchRoomsUpdate(last_timestamp, response) => {
                response
                    .send(Ok(worker.fetch_rooms_update(last_timestamp).await.unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::SendMessage(token, message, response) => {
                response
                    .send(Ok(worker.send_message(message, &token).await.unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::FetchAutocompleteUsers(name, response) => {
                response
                    .send(Ok(worker.fetch_autocomplete_users(&name).await.unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::FetchParticipants(token, response) => {
                response
                    .send(Ok(worker.fetch_participants(&token).await.unwrap()))
                    .expect("could not Send.");
            }
            ApiRequests::MarkChatRead(token, last_message, response) => {
                worker.mark_chat_read(&token, last_message).await.unwrap();
                response.send(Ok(())).expect("could not Send.");
            }
            ApiRequests::None => {
                log::warn!("Unknown Request");
            }
        }
    }
    pub fn new(config: &Config) -> Self {
        let (tx, mut rx) = mpsc::channel::<ApiRequests>(50);

        let mut worker_queue = vec![];

        for i in 1..6 {
            let (tx_worker, mut rx_worker) = mpsc::channel::<ApiRequests>(10);

            worker_queue.push(tx_worker);
            let worker = NCRequestWorker::new(config).unwrap();

            tokio::spawn(async move {
                loop {
                    if let Ok(req) = rx_worker.recv().await {
                        NCRequest::handle_req(&worker, req).await;
                    };
                }
            });
        }

        tokio::spawn(async move {
            loop {
                let mut buffer: Vec<ApiRequests> = vec![];
                let added = rx.recv_many(&mut buffer, 5).await;
                log::debug!("got {} requests to API", added);

                if added == 0 {
                    buffer.push(rx.recv().await.expect("Failed to get message"));
                }

                while worker_queue
                    .first()
                    .expect("No Element in worker queue")
                    .capacity()
                    < 5
                {
                    worker_queue.sort_by_key(tokio::sync::mpsc::Sender::capacity);
                }
                log::debug!(
                    "Capacity of first {} and last {} worker",
                    worker_queue.first().unwrap().capacity(),
                    worker_queue.last().unwrap().capacity()
                );
                for message in buffer {
                    worker_queue
                        .first()
                        .expect("No Thread?")
                        .send(message)
                        .await
                        .expect("Failed to fwd request to worker.");
                }
            }
        });
        log::info!("Spawned API Thread");

        NCRequest { request_tx: tx }
    }
}

#[async_trait]
impl NCRequestInterface for NCRequest {
    async fn request_send_message(
        &self,
        message: String,
        token: &Token,
    ) -> ApiResult<NCReqDataMessage> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.request_tx
            .send(ApiRequests::SendMessage(token.clone(), message, tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_rooms_initial(&self) -> ApiResult<(Vec<NCReqDataRoom>, i64)> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.request_tx
            .send(ApiRequests::FetchRoomsInitial(tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_autocomplete_users(&self, name: &str) -> ApiResult<Vec<NCReqDataUser>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchAutocompleteUsers(name.to_string(), tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_participants(&self, token: &Token) -> ApiResult<Vec<NCReqDataParticipants>> {
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
    ) -> ApiResult<(Vec<NCReqDataRoom>, i64)> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.request_tx
            .send(ApiRequests::FetchRoomsUpdate(last_timestamp, tx))
            .await
            .expect("Queuing request for sending of message failed.");
        Ok(rx)
    }
    async fn request_chat_initial(&self, token: &Token, maxMessage: i32) -> ApiResult<()> {
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
    ) -> ApiResult<Vec<NCReqDataMessage>> {
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
    ) -> ApiResult<NCReqDataMessage>;
    async fn request_autocomplete_users(&self, name: &str) -> ApiResult<Vec<NCReqDataUser>>;
    async fn request_participants(&self, token: &Token) -> ApiResult<Vec<NCReqDataParticipants>>;
    async fn request_rooms_initial(&self) -> ApiResult<(Vec<NCReqDataRoom>, i64)>;
    async fn request_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> ApiResult<(Vec<NCReqDataRoom>, i64)>;
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> ApiResult<Vec<NCReqDataMessage>>;
    async fn request_chat_update(
        &self,
        token: &Token,
        maxMessage: i32,
        last_message: i32,
    ) -> ApiResult<Vec<NCReqDataMessage>>;
    async fn request_mark_chat_read(&self, token: &str, last_message: i32) -> ApiResult<()>;
    }
    impl Clone for NCRequest {   // specification of the trait to mock
        fn clone(&self) -> Self;
    }
}
