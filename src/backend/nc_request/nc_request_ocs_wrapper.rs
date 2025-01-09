//!Wrapper for the OCS API Format all NC APIs use.
use serde::{Deserialize, Serialize};

/// Wrapper for the [OCS API Objects](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/OCS/ocs-api-overview.html#user-metadata)
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCSWrapper<T> {
    /// Contained OCS Tag
    pub ocs: NCReqOCS<T>,
}

/// Inside the OCS we always have metadata and the actual data we want.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCS<T> {
    /// Meta Data
    pub meta: NCReqMeta,
    /// Real Data
    pub data: T,
}

/// Meta Data. Not evaluated here.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqMeta {
    status: String,
    statuscode: i32,
    message: String,
}
