use super::NCReqDataMessage;
use serde::{Deserialize, Deserializer, Serialize};

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
