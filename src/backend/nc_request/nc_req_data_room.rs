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
    pub lastPing: i32,
    pub sessionId: String,
    pub hasPassword: bool,
    pub hasCall: bool,
    pub callFlag: i32,
    pub canStartCall: bool,
    pub canDeleteConversation: bool,
    pub canLeaveConversation: bool,
    pub lastActivity: i32,
    pub isFavorite: bool,
    pub notificationLevel: i32,
    pub lobbyState: i32,
    pub lobbyTimer: i32,
    pub sipEnabled: i32,
    pub canEnableSIP: bool,
    pub unreadMessages: i32,
    pub unreadMention: bool,
    pub unreadMentionDirect: bool,
    pub lastReadMessage: i32,
    pub lastCommonReadMessage: i32,
    #[serde(deserialize_with = "arr_or_message")]
    pub lastMessage: NCReqDataMessage,
    pub objectType: String,
    pub objectId: String,
    pub breakoutRoomMode: i32,
    pub breakoutRoomStatus: i32,
    pub avatarVersion: String,
    pub isCustomAvatar: bool,
    pub callStartTime: i32,
    pub callRecording: i32,
    pub recordingConsent: i32,
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
