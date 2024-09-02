use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NCReqDataParticipants {
    attendeeId: i32,
    actorType: String,
    actorId: String,
    pub displayName: String,
    participantType: i32,
    lastPing: i32,
    inCall: i32,
    permissions: i32,
    attendeePermissions: i32,
    sessionIds: Vec<String>,
    pub status: Option<String>,
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
