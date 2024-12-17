use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCSWrapper<T> {
    pub ocs: NCReqOCS<T>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqOCS<T> {
    pub meta: NCReqMeta,
    pub data: T,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NCReqMeta {
    status: String,
    statuscode: i32,
    message: String,
}
