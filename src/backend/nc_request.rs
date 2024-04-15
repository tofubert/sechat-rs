#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(dead_code)]

use base64::{prelude::BASE64_STANDARD, write::EncoderWriter};
use json;
use reqwest::header::HeaderMap;
use reqwest::{header, Client, Response, Url};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use std::{collections::HashMap, error::Error, fs::File, io::Write};

use crate::config;

#[derive(Debug, Clone)]
pub struct NCRequest {
    base_url: String,
    client: Client,
    base_headers: HeaderMap,
    json_dump_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqMeta {
    status: String,
    statuscode: i32,
    message: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessageParameter {
    #[serde(rename = "type")]
    paramtype: String,
    id: String,
    name: String,
}

fn str_or_status<'de, D>(deserializer: D) -> Result<NCReqDataUserStatus, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageVec {
        ParamMap(Box<NCReqDataUserStatus>),
        String(String),
    }

    Ok(match NCReqDataMessageVec::deserialize(deserializer)? {
        NCReqDataMessageVec::ParamMap(v) => *v, // Ignoring parsing errors
        NCReqDataMessageVec::String(_) => NCReqDataUserStatus::default(),
    })
}

fn arr_or_message<'de, D>(deserializer: D) -> Result<NCReqDataMessage, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageVec {
        ParamMap(Box<NCReqDataMessage>),
        Vec(Vec<i32>),
    }

    Ok(match NCReqDataMessageVec::deserialize(deserializer)? {
        NCReqDataMessageVec::ParamMap(v) => *v, // Ignoring parsing errors
        NCReqDataMessageVec::Vec(_) => NCReqDataMessage::default(),
    })
}

fn arr_or_messageParam<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, NCReqDataMessageParameter>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageParameterMap {
        ParamMap(HashMap<String, NCReqDataMessageParameter>),
        Vec(Vec<i32>),
    }

    Ok(
        match NCReqDataMessageParameterMap::deserialize(deserializer)? {
            NCReqDataMessageParameterMap::ParamMap(v) => v, // Ignoring parsing errors
            NCReqDataMessageParameterMap::Vec(_) => HashMap::new(),
        },
    )
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessageParent {
    pub id: i32,
    pub token: String,
    pub actorType: String,
    pub actorId: String,
    pub actorDisplayName: String,
    pub timestamp: i32,
    pub systemMessage: String,
    pub messageType: String,
    pub isReplyable: bool,
    pub referenceId: String,
    pub message: String,
    #[serde(deserialize_with = "arr_or_messageParam")]
    pub messageParameters: HashMap<String, NCReqDataMessageParameter>,
    pub expirationTimestamp: i32,
    pub reactions: HashMap<String, i32>,
    #[serde(default)]
    pub reactionsSelf: Vec<String>,
    pub markdown: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessage {
    pub id: i32,
    pub token: String,
    pub actorType: String,
    pub actorId: String,
    pub actorDisplayName: String,
    pub timestamp: i64,
    pub systemMessage: String,
    pub messageType: String,
    pub isReplyable: bool,
    pub referenceId: String,
    pub message: String,
    #[serde(deserialize_with = "arr_or_messageParam")]
    pub messageParameters: HashMap<String, NCReqDataMessageParameter>,
    pub expirationTimestamp: i32,
    #[serde(default)]
    pub parent: NCReqDataMessageParent,
    pub reactions: HashMap<String, i32>,
    #[serde(default)]
    pub reactionsSelf: Vec<String>,
    pub markdown: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct NCReqDataRoom {
    pub id: i32,
    pub token: String,
    #[serde(rename = "type")]
    pub roomtype: i32,
    pub name: String,
    pub displayName: String,
    pub description: String,
    pub participantType: i32,
    pub attendeeId: i32,
    pub attendeePin: String,
    pub actorType: String,
    pub actorId: String,
    pub permissions: i32,
    pub attendeePermissions: i32,
    pub callPermissions: i32,
    pub defaultPermissions: i32,
    pub participantFlags: i32,
    pub readOnly: i32,
    pub listable: i32,
    pub messageExpiration: i32,
    lastPing: i32,
    sessionId: String,
    hasPassword: bool,
    hasCall: bool,
    callFlag: i32,
    canStartCall: bool,
    canDeleteConversation: bool,
    canLeaveConversation: bool,
    lastActivity: i32,
    isFavorite: bool,
    notificationLevel: i32,
    lobbyState: i32,
    lobbyTimer: i32,
    sipEnabled: i32,
    canEnableSIP: bool,
    pub unreadMessages: i32,
    unreadMention: bool,
    unreadMentionDirect: bool,
    pub lastReadMessage: i32,
    lastCommonReadMessage: i32,
    #[serde(deserialize_with = "arr_or_message")]
    pub lastMessage: NCReqDataMessage,
    objectType: String,
    objectId: String,
    breakoutRoomMode: i32,
    breakoutRoomStatus: i32,
    avatarVersion: String,
    isCustomAvatar: bool,
    callStartTime: i32,
    callRecording: i32,
    recordingConsent: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataParticipants {
    attendeeId: i32,
    actorType: String,
    actorId: String,
    displayName: String,
    participantType: i32,
    lastPing: i32,
    inCall: i32,
    permissions: i32,
    attendeePermissions: i32,
    sessionIds: Vec<String>,
    status: Option<String>,
    statusIcon: Option<String>,
    statusMessage: Option<String>,
    statusClearAt: Option<i32>,
    roomToken: Option<String>,
    phoneNumber: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataUserStatus {
    status: String,
    message: Option<String>,
    icon: Option<String>,
    clearAt: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataUser {
    id: String,
    label: String,
    icon: String,
    source: String,
    #[serde(deserialize_with = "str_or_status")]
    status: NCReqDataUserStatus,
    subline: String,
    shareWithDisplayNameUnique: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCSWrapper<T> {
    ocs: NCReqOCS<T>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCS<T> {
    meta: NCReqMeta,
    data: T,
}

impl NCRequest {
    pub fn new() -> Result<NCRequest, Box<dyn Error>> {
        let username = config::get().data.general.user.clone();
        let password = Some(config::get().data.general.app_pw.clone());
        let base_url = config::get().data.general.url.clone();
        let json_dump_path = config::get().get_http_dump_dir();
        let mut headers = header::HeaderMap::new();
        headers.insert("OCS-APIRequest", header::HeaderValue::from_static("true"));
        headers.insert(
            "Accept",
            header::HeaderValue::from_static("application/json"),
        );

        let mut buf = b"Basic ".to_vec();
        {
            let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
            let _ = write!(encoder, "{username}:");
            if let Some(password) = password {
                let _ = write!(encoder, "{password}");
            }
        }
        let mut auth_value =
            header::HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        // get a client builder
        let client = reqwest::Client::builder()
            .default_headers(headers.clone())
            .build()?;

        Ok(NCRequest {
            base_url: base_url.to_string(),
            client,
            base_headers: headers,
            json_dump_path,
        })
    }

    pub async fn send_message(
        &self,
        message: String,
        token: &str,
    ) -> Result<NCReqDataMessage, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v1/chat/" + token;
        let mut params = HashMap::new();
        params.insert("message".to_owned(), message.clone());
        let url = Url::parse_with_params(&url_string, params)?;
        let response = self.request_post(url).await?;

        match response.status() {
            reqwest::StatusCode::CREATED => Ok(response
                .json::<NCReqOCSWrapper<NCReqDataMessage>>()
                .await?
                .ocs
                .data),
            _ => Err(Box::new(
                response
                    .error_for_status()
                    .err()
                    .ok_or("Failed to convert Err in reqwest")?,
            )),
        }
    }

    pub async fn fetch_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<Vec<NCReqDataUser>, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/core/autocomplete/get";
        let mut params = HashMap::new();
        params.insert("limit".to_owned(), "200".to_string());
        params.insert("search".to_owned(), name.to_string());

        let url = Url::parse_with_params(&url_string, params)?;

        let response = self.request(url).await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                let text = response.text().await?;
                match serde_json::from_str::<NCReqOCSWrapper<Vec<NCReqDataUser>>>(&text) {
                    Ok(parser_response) => Ok(parser_response.ocs.data),
                    Err(why) => {
                        self.dump_json_to_log(&url_string, &text)?;
                        log::debug!("{} with {:?}", url_string, why);
                        Err(Box::new(why))
                    }
                }
            }
            _ => Err(Box::new(
                response
                    .error_for_status()
                    .err()
                    .ok_or("Failed to convert Err in reqwest")?,
            )),
        }
    }

    pub async fn fetch_participants(
        &self,
        token: &str,
    ) -> Result<Vec<NCReqDataParticipants>, Box<dyn Error>> {
        let url_string = self.base_url.clone()
            + "/ocs/v2.php/apps/spreed/api/v4/room/"
            + token
            + "/participants";
        let mut params = HashMap::new();
        params.insert("includeStatus".to_owned(), "true".to_string());
        let url = Url::parse_with_params(&url_string, params)?;

        let response = self.request(url).await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                let text = response.text().await?;
                match serde_json::from_str::<NCReqOCSWrapper<Vec<NCReqDataParticipants>>>(&text) {
                    Ok(parser_response) => Ok(parser_response.ocs.data),
                    Err(why) => {
                        self.dump_json_to_log(&url_string, &text)?;
                        log::debug!("{} with {:?}", url_string, why);
                        Err(Box::new(why))
                    }
                }
            }
            _ => Err(Box::new(
                response
                    .error_for_status()
                    .err()
                    .ok_or("Failed to convert Err in reqwest")?,
            )),
        }
    }

    pub async fn fetch_rooms_inital(&self) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        self.request_rooms(None).await
    }

    pub async fn fetch_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        self.request_rooms(Some(last_timestamp)).await
    }

    async fn request_rooms(
        &self,
        last_timestamp: Option<i64>,
    ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v4/room";
        let mut params = HashMap::new();
        if let Some(timestamp) = last_timestamp {
            params.insert("modifiedSince".to_owned(), timestamp.to_string());
        }
        let url = Url::parse_with_params(&url_string, &params)?;
        let response = self.request(url).await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                let timestamp = response
                    .headers()
                    .get("X-Nextcloud-Talk-Modified-Before")
                    .ok_or("Failed to get header")?
                    .to_str()?
                    .parse::<i64>()?;
                let text = response.text().await?;
                match serde_json::from_str::<NCReqOCSWrapper<Vec<NCReqDataRoom>>>(&text) {
                    Ok(parser_response) => Ok((parser_response.ocs.data, timestamp)),
                    Err(why) => {
                        self.dump_json_to_log(&url_string, &text)?;
                        Err(Box::new(why))
                    }
                }
            }
            _ => Err(Box::new(
                response
                    .error_for_status()
                    .err()
                    .ok_or("Failed to convert Err in reqwest")?,
            )),
        }
    }

    pub async fn fetch_chat_inital(
        &self,
        token: &str,
        maxMessage: i32,
    ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>> {
        let response_result = self.request_chat(token, maxMessage, None).await;
        // Inital results come last to first. And we want the latest message always to be at the end.
        match response_result {
            Ok(Some(mut response)) => {
                response.reverse();
                Ok(response)
            }
            Ok(None) => Err(String::from("Room disappeared, precondition not met error.").into()),
            Err(why) => Err(why),
        }
    }

    pub async fn fetch_chat_update(
        &self,
        token: &str,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>> {
        let response_result = self
            .request_chat(token, maxMessage, Some(last_message))
            .await;
        match response_result {
            Ok(Some(response)) => Ok(response),
            Ok(None) => Err(String::from("Room disappeared, precondition not met error.").into()),
            Err(why) => Err(why),
        }
    }

    async fn request_chat(
        &self,
        token: &str,
        maxMessage: i32,
        last_message: Option<i32>,
    ) -> Result<Option<Vec<NCReqDataMessage>>, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v1/chat/" + token;
        let mut params = HashMap::new();
        params.insert("limit".to_owned(), maxMessage.to_string());
        params.insert("setReadMarker".to_owned(), "0".to_owned());
        if let Some(lastId) = last_message {
            log::debug!("Last MessageID {}", lastId);
            params.insert("lastKnownMessageId".to_owned(), lastId.to_string());
            params.insert("lookIntoFuture".to_owned(), "1".to_owned());
            params.insert("timeout".to_owned(), "0".to_owned());
            params.insert("includeLastKnown".to_owned(), "0".to_owned());
        } else {
            params.insert("lookIntoFuture".to_owned(), "0".to_owned());
        }
        let url = Url::parse_with_params(&url_string, &params)?;
        let response = self.request(url).await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                log::debug!("Got new Messages.");
                let text = response.text().await?;
                match serde_json::from_str::<NCReqOCSWrapper<Vec<NCReqDataMessage>>>(&text) {
                    Ok(parser_response) => Ok(Some(parser_response.ocs.data)),
                    Err(why) => {
                        self.dump_json_to_log(&url_string, &text)?;
                        Err(Box::new(why))
                    }
                }
            }
            reqwest::StatusCode::NOT_MODIFIED => {
                log::debug!("No new Messages.");
                Ok(Some(Vec::new()))
            }
            reqwest::StatusCode::PRECONDITION_FAILED => Ok(None),
            _ => {
                log::debug!("{} got Err {:?}", token, response);
                Err(Box::new(
                    response
                        .error_for_status()
                        .err()
                        .ok_or("Failed to convert Error")?,
                ))
            }
        }
    }

    pub async fn mark_chat_read(
        &self,
        token: &str,
        last_message: i32,
    ) -> Result<(), Box<dyn Error>> {
        let url_string =
            self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v1/chat/" + token + "/read";
        let url = Url::parse(&url_string)?;
        log::debug!("Marking {} as read", token);
        let response = self.request_post(url).await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => Err(Box::new(
                response
                    .error_for_status()
                    .err()
                    .ok_or("Failed to convert Error")?,
            )),
        }
    }

    async fn request_post(&self, url: Url) -> Result<Response, reqwest::Error> {
        let builder = self.client.post(url);
        builder.send().await
    }

    async fn request(&self, url: Url) -> Result<Response, reqwest::Error> {
        let builder = self.client.get(url);
        builder.send().await
    }

    fn dump_json_to_log(&self, url: &str, text: &str) -> Result<(), Box<dyn Error>> {
        if let Some(path) = &self.json_dump_path {
            let mut name = path.clone();
            name.push(url.replace('/', "_"));
            let mut file = File::create(name)?;
            let pretty_text = json::stringify_pretty(json::parse(text)?, 2);
            file.write_all(pretty_text.as_bytes())?;
        }
        Ok(())
    }
}
