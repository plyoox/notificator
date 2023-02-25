use std::collections::HashMap;

use actix_web::{HttpResponse, ResponseError};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use log::error;

use crate::errors::Error;
use crate::routes::structs::{EventsubCondition, EventsubTransportData, StreamData, TokenExchangeResponse, TwitchApiErrorResponse, TwitchAuthErrorResponse, TwitchEventsub, TwitchStreamsResponse, TwitchUser, TwitchUserResponse};
use crate::structs::{AppState, ErrorResponse, Result};
use crate::utils::current_unix_timestamp;

use self::structs::{AppAccessTokenResponse, CreateTwitchEventsub, EventsubType, TwitchEventsubResponse};

mod structs;
pub mod service;
pub mod twitch;


const TWITCH_API_ENDPOINT: &str = "https://api.twitch.tv/helix";
const TWITCH_AUTH_ENDPOINT: &str = "https://id.twitch.tv";

impl AppState {
    async fn fetch_access_token(&self) -> Result<String> {
        let mut params = HashMap::new();
        params.insert("client_id", self.twitch.client_id);
        params.insert("client_secret", self.twitch.client_secret);
        params.insert("grant_type", "client_credentials");

        let mut res = self.client
            .post(format!("{TWITCH_AUTH_ENDPOINT}/oauth2/token"))
            .send_form(&params)
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<AppAccessTokenResponse>()
                    .await
                    .unwrap();

                let mut data = self.twitch.app_token.lock().map_err(|_| Error::Mutex("Cannot lock context".to_string()))?;

                data.access_token = body.access_token.clone();
                data.expires_at = current_unix_timestamp() + body.expires_in as u64;

                Ok(body.access_token)
            }
            _ => {
                let body = res.body().await.unwrap();
                let text = String::from_utf8(body.to_vec()).map_err(|_| Error::TwitchApi("Received invalid utf8".to_string()))?;
                Err(Error::TwitchApi(text))
            }
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        let token = {
            let token_mutex = self.twitch.app_token
                .lock()
                .map_err(|_| Error::Mutex("Cannot lock mutex".to_string()))?;

            token_mutex.clone()
        };

        if !token.access_token.is_empty() && token.expires_at >= current_unix_timestamp() {
            Ok(token.access_token.clone())
        } else {
            self.fetch_access_token().await
        }
    }

    pub async fn fetch_user(&self, token: &str) -> Result<TwitchUser> {
        let mut res = self.client
            .post(format!("{TWITCH_API_ENDPOINT}/users"))
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let res_data = res.json::<TwitchUserResponse>().await.unwrap();
                if res_data.data.is_empty() {
                    Err(Error::TwitchApi("Received empty user response".to_string()))
                } else {
                    Ok(res_data.data[0].clone())
                }
            }
            400 => {
                Err(Error::InternalServer("Invalid request when fetching users".to_string()))
            }
            401 => {
                Err(Error::TwitchApi("Invalid authorization used".to_string()))
            }
            c => {
                error!(target: "twitch", "Received unknown status code: {c}");
                Err(Error::TwitchApi("Received unknown status code".to_string()))
            }
        }
    }


    pub async fn fetch_user_token(&self, code: &str) -> Result<String> {
        let mut params = HashMap::new();
        params.insert("client_id", self.twitch.client_id);
        params.insert("client_secret", self.twitch.client_secret);
        params.insert("redirect_uri", self.twitch.redirect_url);
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);

        let url = format!("{TWITCH_AUTH_ENDPOINT}/oauth2/token");
        let mut res = self.client
            .post(url.as_str())
            .send_form(&params)
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TokenExchangeResponse>().await.unwrap();

                Ok(body.access_token)
            }
            c => {
                let res_data = res.json::<TwitchAuthErrorResponse>().await.unwrap();

                error!(target: "twitch", "POST {} resulted in {c}: {}", url.as_str(), res_data.message);
                Err(Error::InternalServer("An error occurred while fetching an eventsub".to_string()))
            }
        }
    }

    async fn fetch_eventsub_by_user(&self, user_id: i32) -> Result<Option<TwitchEventsub>> {
        let app_token = self.fetch_access_token().await?;

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions?user_id={user_id}");
        let mut res = self.client
            .get(url.as_str())
            .insert_header(("Client-Id", self.twitch.client_id))
            .bearer_auth(app_token)
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TwitchEventsubResponse>().await.unwrap();

                Ok(
                    if body.data.is_empty() {
                        None
                    } else {
                        Some(body.data[0].clone())
                    }
                )
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();

                error!(target: "twitch", "GET {} resulted in {c}: {}", url.as_str(), res_data.message);
                Err(Error::InternalServer("An error occurred while fetching an eventsub".to_string()))
            }
        }
    }

    pub async fn register_eventsub(&self, token: &str, user_id: i32) -> Result<String> {
        let body = CreateTwitchEventsub {
            event_type: EventsubType::StreamOnline,
            version: "1".to_string(),
            condition: EventsubCondition { broadcaster_user_id: user_id.to_string() },
            transport: EventsubTransportData {
                callback: self.twitch.callback_url.to_owned(),
                secret: Some(self.twitch.eventsub_secret.to_owned()),
                method: "webhook".to_owned(),
            },
        };

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions");
        let mut res = self.client
            .post(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send_json(&body)
            .await?;

        match res.status().as_u16() {
            202 => {
                let body = res.json::<TwitchEventsubResponse>().await.unwrap();
                let event_sub = body.data.first().unwrap();

                Ok(event_sub.id.clone())
            }
            409 => {
                let subscription = self.fetch_eventsub_by_user(user_id).await?;

                if let Some(s) = subscription {
                    Ok(s.id)
                } else {
                    error!(target: "twitch", "Cannot find existing eventsub for user {user_id}");
                    Err(Error::TwitchApi("Cannot find existing eventsub for user".to_string()))
                }
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();

                error!(target: "twitch", "POST {} resulted in {c}: {}", url.as_str(), res_data.message);
                Err(Error::InternalServer("An error occurred while registering an eventsub".to_string()))
            }
        }
    }

    pub async fn delete_eventsub(&self, id: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions?id={id}");
        let mut res = self.client
            .delete(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            204 => {
                Ok(())
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();
                error!(target: "twitch", "DELETE {} resulted in {c}: {}", url.as_str(), res_data.message);

                Err(Error::InternalServer("An error occurred while deleting an eventsub.".to_string()))
            }
        }
    }

    pub async fn fetch_stream_data(&self, user_id: i32) -> Result<StreamData> {
        let token = self.get_access_token().await?;

        let url = format!("https://api.twitch.tv/helix/streams?user_id={user_id}");
        let mut res = self.client
            .get(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TwitchStreamsResponse>().await.unwrap();

                if body.data.is_empty() {
                    Err(Error::TwitchApi("No stream data returned.".to_string()))
                } else {
                    Ok(body.data[0].clone())
                }
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();
                error!(target: "twitch", "GET {} resulted in {c}: {}", url.as_str(), res_data.message);

                Err(Error::InternalServer("An error occurred while fetching a stream.".to_string()))
            }
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Error::Awc(e) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("Request Error: {e}"),
                })
            }
            Error::TwitchApi(e) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("Twitch API Error: {e}"),
                })
            }
            Error::InternalServer(e) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: e.to_string(),
                })
            }
            Error::Mutex(e) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("Request Error: {e}"),
                })
            }
            Error::SQLx(e) => {
                error!(target: "database", "An error occurred while executing a database query: {e}");

                HttpResponse::InternalServerError().json(ErrorResponse {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "An error occurred while executing a database query".to_string(),
                })
            }
            Error::BadRequest(e) => {
                HttpResponse::BadRequest().json(ErrorResponse {
                    code: StatusCode::BAD_REQUEST,
                    message: e.to_string(),
                })
            }
        }
    }
}