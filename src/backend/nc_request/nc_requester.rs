//! The Requester is the Sechat facing Abstraction of the NC API.
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use async_trait::async_trait;

use std::{error::Error, fmt};
use std::{fmt::Debug, sync::Arc};

#[cfg(test)]
use mockall::{mock, predicate::*};

use super::{
    nc_req_worker::{NCRequestWorker, NCRequestWorkerInterface},
    NCReqDataMessage, NCReqDataParticipants, NCReqDataRoom, NCReqDataUser, Token,
};

type ApiResult<T> =
    Result<oneshot::Receiver<Result<T, Arc<dyn Error + Send + Sync>>>, Box<dyn Error>>;
type ApiResponseChannel<T> = oneshot::Sender<Result<T, Arc<dyn Error + Send + Sync>>>;

#[derive(Default, Debug)]
pub enum ApiRequests {
    #[default]
    None,
    SendMessage(Token, String, ApiResponseChannel<NCReqDataMessage>),
    FetchRoomsInitial(ApiResponseChannel<(Vec<NCReqDataRoom>, i64)>),
    FetchRoomsUpdate(i64, ApiResponseChannel<(Vec<NCReqDataRoom>, i64)>),
    FetchParticipants(Token, ApiResponseChannel<Vec<NCReqDataParticipants>>),
    FetchChatInitial(Token, i32, ApiResponseChannel<Vec<NCReqDataMessage>>),
    FetchChatUpdate(Token, i32, i32, ApiResponseChannel<Vec<NCReqDataMessage>>),
    FetchAutocompleteUsers(String, ApiResponseChannel<Vec<NCReqDataUser>>),
    MarkChatRead(Token, i32, ApiResponseChannel<()>),
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
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// The [`NCRequest`] uses a number of Threads to distribute the Requests to the Workers.
///
/// To Communicate with the Threads a List of [`tokio::sync::mpsc`] channels is used.
/// The Threads get passed a [`ApiRequests`] with Response Channel as a payload.
/// A User of this API can then Poll on the [`ApiResponseChannel`]
#[derive(Debug)]
pub struct NCRequest {
    request_tx: Sender<ApiRequests>,
    cancel_token: CancellationToken,
}

impl NCRequest {
    async fn handle_req(worker: &NCRequestWorker, req: ApiRequests) {
        log::trace!("got a new API Request {}", req);
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
        let cancel_token = CancellationToken::new();

        for i in 1..6 {
            let cloned_cancel_token = cancel_token.clone();

            let (tx_worker, mut rx_worker) = mpsc::channel::<ApiRequests>(10);

            worker_queue.push(tx_worker);
            let worker = NCRequestWorker::new(config).expect("Failed to create worker.");

            tokio::spawn(async move {
                while !cloned_cancel_token.is_cancelled() {
                    if let Some(req) = rx_worker.recv().await {
                        NCRequest::handle_req(&worker, req).await;
                    };
                }
            });
        }
        let cloned_cancel_token = cancel_token.clone();

        tokio::spawn(async move {
            while !cloned_cancel_token.is_cancelled() {
                let mut buffer: Vec<ApiRequests> = vec![];
                let added = rx.recv_many(&mut buffer, 5).await;
                log::trace!("got {} requests to API", added);

                // the revc_many function might be in flight while we get cancelt.
                if cloned_cancel_token.is_cancelled() {
                    break;
                }

                if added == 0 {
                    buffer.push(rx.recv().await.expect("Failed to get message"));
                }

                if worker_queue
                    .first()
                    .expect("No Element in worker queue")
                    .capacity()
                    < 5
                {
                    log::trace!(
                        "Capacity of first {} and last {} worker. Rotating",
                        worker_queue.first().unwrap().capacity(),
                        worker_queue.last().unwrap().capacity()
                    );
                    worker_queue.rotate_right(1);
                }

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

        NCRequest {
            request_tx: tx,
            cancel_token,
        }
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
    async fn request_chat_initial(
        &self,
        token: &Token,
        maxMessage: i32,
    ) -> ApiResult<Vec<NCReqDataMessage>> {
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
    async fn request_mark_chat_read(&self, token: &str, last_message: i32) -> ApiResult<()> {
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
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cancel_token.cancel();
        Ok(())
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
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
    }
    impl Clone for NCRequest {   // specification of the trait to mock
        fn clone(&self) -> Self;
    }
}

#[cfg(test)]
mod tests {

    use crate::config::init;

    use super::*;

    #[tokio::test]
    async fn create() {
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var("HOME", dir.path().as_os_str());
        let config = init("./test/").unwrap();

        let requester = NCRequest::new(&config);
    }
}
