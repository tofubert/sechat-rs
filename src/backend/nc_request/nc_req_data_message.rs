use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use strum::Display;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataMessageParameter {
    #[serde(deserialize_with = "message_param_type")]
    #[serde(rename = "type")]
    pub param_type: NCReqDataMessageParameterType,
    pub id: String,
    pub name: String,
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
    #[serde(deserialize_with = "message_type")]
    pub messageType: NCReqDataMessageType,
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
    #[serde(deserialize_with = "message_type")]
    pub messageType: NCReqDataMessageType,
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

/// System Messages in NC dont seem to have a collected global state, but seem to be defined by anyone and everyone.
///
/// This is collected and greped out of various log files and the spreed source code.
/// Please help extend this.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Display)]
pub enum NCReqDataMessageSystemMessage {
    #[default]
    #[serde(rename = "")]
    Nomessage,
    #[serde(rename = "message_edited")]
    MessageEdited,
    #[serde(rename = "message_deleted")]
    MessageDeleted,
    #[serde(rename = "reaction")]
    Reaction,
    #[serde(rename = "reaction_revoked")]
    ReactionRevoked,
    #[serde(rename = "reaction_deleted")]
    ReactionDeleted,
    #[serde(rename = "history_cleared")]
    HistoryCleared,
    #[serde(rename = "poll_voted")]
    PollVoted,
    #[serde(rename = "poll_closed")]
    PollClosed,
    #[serde(rename = "call_started")]
    CallStarted,
    #[serde(rename = "call_ended")]
    CallEnded,
    #[serde(rename = "call_ended_everyone")]
    CallEndedEveryone,
    #[serde(rename = "call_missed")]
    CallMissed,
    #[serde(rename = "call_joined")]
    CallJoined,
    #[serde(rename = "call_left")]
    CallLeft,
    #[serde(rename = "user_removed")]
    UserRemoved,
    #[serde(rename = "user_added")]
    UserAdded,
    #[serde(rename = "listable_users")]
    ListableUsers,
    #[serde(rename = "avatar_set")]
    AvatarSet,
    #[serde(rename = "conversation_renamed")]
    ConversationRenamed,
    #[serde(rename = "conversation_created")]
    ConversationCreated,
    #[serde(rename = "read_only")]
    ReadOnly,
    #[serde(rename = "listable_none")]
    ListableNone,
    #[serde(rename = "group_added")]
    GroupAdded,
    #[serde(rename = "group_removed")]
    GroupRemoved,
    #[serde(rename = "description_set")]
    DescriptionSet,
    #[serde(rename = "moderator_promoted")]
    ModeratorPromoted,
    #[serde(rename = "matterbridge_config_enabled")]
    MatterbridgeConfigEnabled,
    #[serde(rename = "matterbridge_config_disabled")]
    MatterbridgeConfigDisabled,
    #[serde(rename = "matterbridge_config_edited")]
    MatterbridgeConfigEdited,
    #[serde(rename = "i_am_the_system")]
    IAmTheSystem,
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
                log::warn!("unknown System Message {}", s);
                NCReqDataMessageSystemMessage::Nomessage
            }
        },
    )
}

/// Message Type defined in [NCTalk API](<https://nextcloud-talk.readthedocs.io/en/stable/chat/#receive-chat-messages-of-a-conversation>)
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Display)]
pub enum NCReqDataMessageType {
    #[default]
    #[serde(rename = "")]
    Unknown,
    #[serde(rename = "comment")]
    Comment,
    #[serde(rename = "comment_deleted")]
    CommentDeleted,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "command")]
    Command,
}

fn message_type<'de, D>(deserializer: D) -> Result<NCReqDataMessageType, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageTypeMap {
        ParamMap(NCReqDataMessageType),
        String(String),
    }

    Ok(match NCReqDataMessageTypeMap::deserialize(deserializer)? {
        NCReqDataMessageTypeMap::ParamMap(v) => v, // Ignoring parsing errors
        NCReqDataMessageTypeMap::String(s) => {
            log::warn!("unknown Message type {}", s);
            NCReqDataMessageType::Unknown
        }
    })
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum NCReqDataMessageParameterType {
    #[default]
    Unknown,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "call")]
    Call,
    #[serde(rename = "guest")]
    Guest,
    #[serde(rename = "talk-poll")]
    TalkPoll,
    #[serde(rename = "highlight")]
    Highlight,
}

fn message_param_type<'de, D>(deserializer: D) -> Result<NCReqDataMessageParameterType, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NCReqDataMessageParamTypeMap {
        ParamMap(NCReqDataMessageParameterType),
        String(String),
    }

    Ok(
        match NCReqDataMessageParamTypeMap::deserialize(deserializer)? {
            NCReqDataMessageParamTypeMap::ParamMap(v) => v, // Ignoring parsing errors
            NCReqDataMessageParamTypeMap::String(s) => {
                log::warn!("unknown Message Param type {}", s);
                NCReqDataMessageParameterType::Unknown
            }
        },
    )
}
