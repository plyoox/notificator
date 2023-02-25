use std::fmt::Debug;

use log::warn;

#[derive(Debug, derive_more::Display)]
pub enum Error {
    #[display(fmt = "Error sending http request {}", _0)]
    Awc(awc::error::SendRequestError),
    #[display(fmt = "Twitch API returned an error: {}", _0)]
    Twitch(String),
    #[display(fmt = "{}", _0)]
    InternalServer(String),
    #[display(fmt = "Could not lock mutex")]
    Mutex,
    #[display(fmt = "Error executing database query: {:?}", _0)]
    SQLx(sqlx::Error),
    BadRequest(String),
    #[display(fmt = "Notification already exists")]
    Conflict
}

impl From<awc::error::SendRequestError> for Error {
    fn from(value: awc::error::SendRequestError) -> Self {
        warn!(target: "http", "Error sending http request: {value:?}");
        Self::Awc(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        warn!(target: "sql", "Error executing database query: {value:?}");
        Self::SQLx(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        warn!("Could not parse JSON body: {}", value.to_string());
        Self::BadRequest("Cannot parse given body.".to_string())
    }
}
