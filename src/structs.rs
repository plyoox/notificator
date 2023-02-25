use std::sync::Mutex;

use actix_web::http::StatusCode;
use sqlx::PgPool;

use crate::errors::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub struct AppState {
    pub twitch: TwitchState,
    pub db: PgPool,
    pub bot_url: &'static str,
    pub client: awc::Client,
}

pub struct TwitchState {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_url: &'static str,
    pub callback_url: &'static str,
    pub eventsub_secret: &'static str,
    pub app_token: Mutex<TwitchAccessToken>,
}

#[derive(Clone)]
pub struct TwitchAccessToken {
    pub access_token: String,
    pub expires_at: u64,
}

pub struct ErrorResponse {
    pub(crate) code: StatusCode,
    pub(crate) message: String,
}
