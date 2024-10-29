#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod nc_req_data_message;
mod nc_req_data_room;
mod nc_req_data_user;
mod nc_request_ocs_wrapper;

pub use nc_req_data_message::*;
pub use nc_req_data_room::*;
pub use nc_req_data_user::*;
pub use nc_request_ocs_wrapper::*;

use crate::config;
use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, write::EncoderWriter};
use jzon;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, Response, Url,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{collections::HashMap, error::Error};

#[cfg(test)]
use mockall::{mock, predicate::*};

#[async_trait]
pub trait NCRequestInterface: Debug + Send + Clone + Default + Send + Sync {
    async fn send_message(
        &self,
        message: String,
        token: &str,
    ) -> Result<NCReqDataMessage, Box<dyn Error>>;
    async fn fetch_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<Vec<NCReqDataUser>, Box<dyn Error>>;
    async fn fetch_participants(
        &self,
        token: &str,
    ) -> Result<Vec<NCReqDataParticipants>, Box<dyn Error>>;
    async fn fetch_rooms_initial(&self) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>;
    async fn fetch_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>;
    async fn fetch_chat_initial(
        &self,
        token: &str,
        maxMessage: i32,
    ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>>;
    async fn fetch_chat_update(
        &self,
        token: &str,
        maxMessage: i32,
        last_message: i32,
    ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>>;
    async fn mark_chat_read(&self, token: &str, last_message: i32) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Clone, Default)]
pub struct NCRequest {
    base_url: String,
    client: Client,
    base_headers: HeaderMap,
    json_dump_path: Option<std::path::PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessageParameter {
    #[serde(rename = "type")]
    param_type: String,
    id: String,
    name: String,
}

impl NCRequest {
    pub fn new() -> Result<NCRequest, Box<dyn Error>> {
        use std::io::Write;

        let config = &config::get();
        let general = &config.data.general;

        let username = general.user.clone();
        let password = Some(general.app_pw.clone());
        let base_url = general.url.clone();

        let json_dump_path = config.get_http_dump_dir();
        let mut headers = HeaderMap::new();
        headers.insert("OCS-APIRequest", HeaderValue::from_static("true"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let mut buf = b"Basic ".to_vec();
        {
            let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
            write!(encoder, "{username}:").expect("i/o error");
            if let Some(password) = password {
                write!(encoder, "{password}").expect("i/o error");
            }
        }
        let mut auth_value =
            HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);

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

    async fn request_rooms(
        &self,
        last_timestamp: Option<i64>,
    ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v4/room";
        let params = if let Some(timestamp) = last_timestamp {
            HashMap::from([("modifiedSince", timestamp.to_string())])
        } else {
            HashMap::new()
        };
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

    async fn request_chat(
        &self,
        token: &str,
        maxMessage: i32,
        last_message: Option<i32>,
    ) -> Result<Option<Vec<NCReqDataMessage>>, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v1/chat/" + token;
        let params = if let Some(lastId) = last_message {
            log::debug!("Last MessageID {}", lastId);
            HashMap::from([
                ("limit", maxMessage.to_string()),
                ("setReadMarker", "0".into()),
                ("lookIntoFuture", "1".into()),
                ("lastKnownMessageId", lastId.to_string()),
                ("timeout", "0".into()),
                ("includeLastKnown", "0".into()),
            ])
        } else {
            HashMap::from([
                ("limit", maxMessage.to_string()),
                ("setReadMarker", "0".into()),
                ("lookIntoFuture", "0".into()),
            ])
        };
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

    async fn request_post(&self, url: Url) -> Result<Response, reqwest::Error> {
        let builder = self.client.post(url);
        builder.send().await
    }

    async fn request(&self, url: Url) -> Result<Response, reqwest::Error> {
        let builder = self.client.get(url);
        builder.send().await
    }

    fn dump_json_to_log(&self, url: &str, text: &str) -> Result<(), Box<dyn Error>> {
        use std::io::Write;

        if let Some(path) = &self.json_dump_path {
            let name: String = url
                .chars()
                .map(|ch| if ch == '/' { '_' } else { ch })
                .collect();
            let mut file = std::fs::File::create(name)?;
            let pretty_text = jzon::stringify_pretty(jzon::parse(text)?, 2);
            file.write_all(pretty_text.as_bytes())?;
        }
        Ok(())
    }
}

#[async_trait]
impl NCRequestInterface for NCRequest {
    async fn send_message(
        &self,
        message: String,
        token: &str,
    ) -> Result<NCReqDataMessage, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/apps/spreed/api/v1/chat/" + token;
        let params = HashMap::from([("message", message)]);
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

    async fn fetch_autocomplete_users(
        &self,
        name: &str,
    ) -> Result<Vec<NCReqDataUser>, Box<dyn Error>> {
        let url_string = self.base_url.clone() + "/ocs/v2.php/core/autocomplete/get";
        let params = HashMap::from([("limit", "200"), ("search", name)]);
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

    async fn fetch_participants(
        &self,
        token: &str,
    ) -> Result<Vec<NCReqDataParticipants>, Box<dyn Error>> {
        let url_string = self.base_url.clone()
            + "/ocs/v2.php/apps/spreed/api/v4/room/"
            + token
            + "/participants";
        let params = HashMap::from([("includeStatus", "true")]);
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

    async fn fetch_rooms_initial(&self) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        self.request_rooms(None).await
    }

    async fn fetch_rooms_update(
        &self,
        last_timestamp: i64,
    ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>> {
        self.request_rooms(Some(last_timestamp)).await
    }

    async fn fetch_chat_initial(
        &self,
        token: &str,
        maxMessage: i32,
    ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>> {
        let response_result = self.request_chat(token, maxMessage, None).await;
        // Initial results come last to first. And we want the latest message always to be at the end.
        match response_result {
            Ok(Some(mut response)) => {
                response.reverse();
                Ok(response)
            }
            Ok(None) => Err(String::from("Room disappeared, precondition not met error.").into()),
            Err(why) => Err(why),
        }
    }

    async fn fetch_chat_update(
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

    async fn mark_chat_read(&self, token: &str, last_message: i32) -> Result<(), Box<dyn Error>> {
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
}

#[cfg(test)]
mock! {
    #[derive(Debug, Default, Clone)]
    pub NCRequest {}     // Name of the mock struct, less the "Mock" prefix

    #[async_trait]
    impl NCRequestInterface for NCRequest {
        async fn send_message(
            &self,
            message: String,
            token: &str,
        ) -> Result<NCReqDataMessage, Box<dyn Error>>;
        async fn fetch_autocomplete_users(
            &self,
            name: &str,
        ) -> Result<Vec<NCReqDataUser>, Box<dyn Error>>;
        async fn fetch_participants(
            &self,
            token: &str,
        ) -> Result<Vec<NCReqDataParticipants>, Box<dyn Error>>;
        async fn fetch_rooms_initial(&self) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>;
        async fn fetch_rooms_update(
            &self,
            last_timestamp: i64,
        ) -> Result<(Vec<NCReqDataRoom>, i64), Box<dyn Error>>;
        async fn fetch_chat_initial(
            &self,
            token: &str,
            maxMessage: i32,
        ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>>;
        async fn fetch_chat_update(
            &self,
            token: &str,
            maxMessage: i32,
            last_message: i32,
        ) -> Result<Vec<NCReqDataMessage>, Box<dyn Error>>;
        async fn mark_chat_read(&self, token: &str, last_message: i32) -> Result<(), Box<dyn Error>>;
    }
    impl Clone for NCRequest {   // specification of the trait to mock
        fn clone(&self) -> Self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_requester() {
        let _ = config::init("./test/");
        let result = NCRequest::new();
        assert!(result.is_ok());
        let requester = result.unwrap();
    }
}
