use std::collections::HashMap;

use log::{error, warn};

use crate::errors::Error;
use crate::structs::{AppState, Result};
use crate::utils::current_unix_timestamp;

use self::structs::{
    AppAccessTokenResponse, CreateTwitchEventsub, EventsubType, TwitchEventsubResponse,
};
use self::structs::{
    EventsubCondition, EventsubTransportData, StreamData, TokenExchangeResponse,
    TwitchApiErrorResponse, TwitchAuthErrorResponse, TwitchEventsub, TwitchStreamsResponse,
    TwitchUser, TwitchUserResponse,
};

pub mod auth;
pub mod service;
pub mod structs;

const TWITCH_API_ENDPOINT: &str = "https://api.twitch.tv/helix";
const TWITCH_AUTH_ENDPOINT: &str = "https://id.twitch.tv";

impl AppState {
    async fn fetch_access_token(&self) -> Result<String> {
        let mut params = HashMap::new();
        params.insert("client_id", self.twitch.client_id);
        params.insert("client_secret", self.twitch.client_secret);
        params.insert("grant_type", "client_credentials");

        let mut res = self
            .client
            .post(format!("{TWITCH_AUTH_ENDPOINT}/oauth2/token"))
            .send_form(&params)
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<AppAccessTokenResponse>().await.unwrap();

                let mut data = self.twitch.app_token.lock().map_err(|_| Error::Mutex)?;

                data.access_token = body.access_token.clone();
                data.expires_at = current_unix_timestamp() + body.expires_in as u64;

                Ok(body.access_token)
            }
            _ => {
                let body = res.body().await.unwrap();
                let text = String::from_utf8(body.to_vec())
                    .map_err(|_| Error::Twitch("Received invalid utf8".to_string()))?;
                Err(Error::Twitch(text))
            }
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        let token = {
            let token_mutex = self.twitch.app_token.lock().map_err(|_| Error::Mutex)?;

            token_mutex.clone()
        };

        if !token.access_token.is_empty() && token.expires_at >= current_unix_timestamp() {
            Ok(token.access_token.clone())
        } else {
            self.fetch_access_token().await
        }
    }

    pub async fn fetch_user(&self, token: &str) -> Result<TwitchUser> {
        let url = format!("{TWITCH_API_ENDPOINT}/users");
        let mut res = self
            .client
            .get(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let res_data = res.json::<TwitchUserResponse>().await.unwrap();
                Ok(res_data.data[0].clone())
            }
            400 => Err(Error::InternalServer(
                "Bad request while fetching users".to_string(),
            )),
            401 => Err(Error::Twitch("Invalid authorization used".to_string())),
            c => {
                let res_data = res.json::<TwitchAuthErrorResponse>().await.unwrap();
                error!(target: "twitch", "GET {url} resulted in {c}: {res_data:?}");

                Err(Error::InternalServer(
                    "Received unhandled status code".to_string(),
                ))
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
        let mut res = self.client.post(url.as_str()).send_form(&params).await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TokenExchangeResponse>().await.unwrap();

                Ok(body.access_token)
            }
            c => {
                let res_data = res.json::<TwitchAuthErrorResponse>().await.unwrap();
                error!(target: "twitch", "POST {} resulted in {c}: {res_data:?}", url.as_str());

                Err(Error::InternalServer(
                    "An error occurred while fetching an eventsub".to_string(),
                ))
            }
        }
    }

    async fn fetch_eventsub_by_user(&self, user_id: i32) -> Result<Option<TwitchEventsub>> {
        let app_token = self.fetch_access_token().await?;

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions?user_id={user_id}");
        let mut res = self
            .client
            .get(url.as_str())
            .insert_header(("Client-Id", self.twitch.client_id))
            .bearer_auth(app_token)
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TwitchEventsubResponse>().await.unwrap();

                Ok(if body.data.is_empty() {
                    None
                } else {
                    Some(body.data[0].clone())
                })
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();
                error!(target: "twitch", "GET {} resulted in {c}: {res_data:?}", url.as_str());

                Err(Error::InternalServer(
                    "An error occurred while fetching an eventsub".to_string(),
                ))
            }
        }
    }

    pub async fn register_eventsub(&self, user_id: i32) -> Result<String> {
        let token = self.get_access_token().await?;

        let body = CreateTwitchEventsub {
            event_type: EventsubType::StreamOnline,
            version: "1".to_string(),
            condition: EventsubCondition {
                broadcaster_user_id: user_id.to_string(),
            },
            transport: EventsubTransportData {
                callback: self.twitch.callback_url.to_owned(),
                secret: Some(self.twitch.eventsub_secret.to_owned()),
                method: "webhook".to_owned(),
            },
        };

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions");
        let mut res = self
            .client
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
                    Err(Error::Twitch(
                        "Cannot find existing eventsub for user".to_string(),
                    ))
                }
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();

                error!(target: "twitch", "POST {} resulted in {c}: {res_data:?}", url.as_str());
                Err(Error::InternalServer(
                    "An error occurred while registering an eventsub".to_string(),
                ))
            }
        }
    }

    pub async fn delete_eventsub(&self, id: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!("{TWITCH_API_ENDPOINT}/eventsub/subscriptions?id={id}");
        let mut res = self
            .client
            .delete(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            204 => Ok(()),
            404 => {
                warn!(target: "twitch", "Eventsub with {id} not found");

                Ok(())
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();
                error!(target: "twitch", "DELETE {} resulted in {c}: {res_data:?}", url.as_str());

                Err(Error::InternalServer(
                    "Twitch response is not handled".to_string(),
                ))
            }
        }
    }

    pub async fn fetch_stream_data(&self, user_id: i32) -> Result<StreamData> {
        let token = self.get_access_token().await?;

        let url = format!("https://api.twitch.tv/helix/streams?user_id={user_id}");
        let mut res = self
            .client
            .get(url.as_str())
            .bearer_auth(token)
            .insert_header(("Client-Id", self.twitch.client_id))
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let body = res.json::<TwitchStreamsResponse>().await.unwrap();

                if body.data.is_empty() {
                    Err(Error::Twitch("No stream data returned.".to_string()))
                } else {
                    Ok(body.data[0].clone())
                }
            }
            c => {
                let res_data = res.json::<TwitchApiErrorResponse>().await.unwrap();
                error!(target: "twitch", "GET {} resulted in {c}: {res_data:?}", url.as_str());

                Err(Error::InternalServer(
                    "An error occurred while fetching a stream.".to_string(),
                ))
            }
        }
    }
}
