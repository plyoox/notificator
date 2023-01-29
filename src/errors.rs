use std::fmt::Debug;

use awc::error::SendRequestError;
use log::warn;

#[derive(Debug, derive_more::Display)]
pub enum Error {
    Awc(String),
    TwitchApi(String),
    InternalServer(String),
    Mutex(String),
    SQLx(String),
    BadRequest(String),
}

impl From<SendRequestError> for Error {
    fn from(value: SendRequestError) -> Self {
        warn!("Error while sending web request: {value:?}");
        Self::Awc(value.to_string())
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        warn!("Error while executing database query: {value:?}");
        Self::SQLx(value.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        warn!("Could not parse JSON body: {}", value.to_string());
        Self::BadRequest("Cannot parse given body.".to_string())
    }
}