use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use strum::Display;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessageParameter {
    #[serde(rename = "type")]
    param_type: String,
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessage {
    pub id: i32,
    pub token: String,
    pub actorType: String,
    pub actorId: String,
    pub actorDisplayName: String,
    pub timestamp: i64,
    #[serde(deserialize_with = "sys_Message")]
    pub systemMessage: NCReqDataMessageSystemMessage,
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
pub struct NCReqDataMessageParent {
    pub id: i32,
    pub token: String,
    pub actorType: String,
    pub actorId: String,
    pub actorDisplayName: String,
    pub timestamp: i32,
    #[serde(deserialize_with = "sys_Message")]
    pub systemMessage: NCReqDataMessageSystemMessage,
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Display)]
pub enum NCReqDataMessageSystemMessage {
    #[default]
    #[serde(rename = "")]
    nomessage,
    message_edited,
    message_deleted,
    reaction,
    reaction_revoked,
    reaction_deleted,
    history_cleared,
    poll_voted,
    poll_closed,
    call_started,
    call_ended,
    call_ended_everyone,
    call_missed,
    call_joined,
    call_left,
    user_removed,
    user_added,
    avatar_set,
    conversation_renamed,
    conversation_created,
    read_only,
    listable_none,
    group_added,
    moderator_promoted,
    matterbridge_config_enabled,
    matterbridge_config_disabled,
    matterbridge_config_edited,
    i_am_the_system,
}

fn sys_Message<'de, D>(deserializer: D) -> Result<NCReqDataMessageSystemMessage, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageSystemMessageMap {
        ParamMap(NCReqDataMessageSystemMessage),
        String(String),
    }

    Ok(
        match NCReqDataMessageSystemMessageMap::deserialize(deserializer)? {
            NCReqDataMessageSystemMessageMap::ParamMap(v) => v, // Ignoring parsing errors
            NCReqDataMessageSystemMessageMap::String(s) => {
                log::warn!("unkowen System Message {}", s);
                NCReqDataMessageSystemMessage::nomessage
            }
        },
    )
}
