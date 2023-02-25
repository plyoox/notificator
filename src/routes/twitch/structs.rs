use std::fmt::Display;
use std::str::FromStr;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use validator::Validate;

use crate::structs::ErrorResponse;

#[derive(Serialize)]
struct RegisterEventsubData {
    pub version: &'static str,
    #[serde(rename = "type")]
    pub event_type: EventsubType,
    pub transport: EventsubTransportData,
    pub condition: EventsubCondition,
}

#[derive(Deserialize)]
pub struct TwitchEventsubResponse {
    pub data: Vec<TwitchEventsub>,
}

#[derive(Deserialize, Clone)]
pub struct TwitchEventsub {
    pub id: String,
    pub status: String,
    #[serde(rename = "type")]
    pub event_type: EventsubType,
    pub version: String,
    pub condition: EventsubCondition,
    pub created_at: String,
    pub transport: EventsubTransportData,
    pub cost: u16,
}

#[derive(Serialize)]
pub struct CreateTwitchEventsub {
    #[serde(rename = "type")]
    pub event_type: EventsubType,
    pub version: String,
    pub condition: EventsubCondition,
    pub transport: EventsubTransportData,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum EventsubType {
    #[serde(rename = "stream.online")]
    StreamOnline,
    #[serde(rename = "WebhookCallbackVerificationPending")]
    WebhookCallbackVerificationPending,
    #[serde(rename = "webhook_callback_verification_failed")]
    WebhookCallbackVerificationFailed,
    #[serde(rename = "notification_failures_exceeded")]
    NotificationFailuresExceeded,
    #[serde(rename = "user_removed")]
    UserRemoved,
    #[serde(rename = "authorization_revoked")]
    AuthorizationRevoked,
}

#[derive(Deserialize)]
pub enum EventsubStatus {
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "webhook_callback_verification_pending")]
    WebhookCallbackVerificationPending,
    #[serde(rename = "webhook_callback_verification_failed")]
    WebhookCallbackVerificationFailed,
    #[serde(rename = "notification_failures_exceeded")]
    NotificationFailuresExceeded,
    #[serde(rename = "authorization_revoked")]
    AuthorizationRevoked,
    #[serde(rename = "moderator_removed")]
    ModeratorRemoved,
    #[serde(rename = "user_removed")]
    UserRemoved,
    #[serde(rename = "version_removed")]
    VersionRemoved,
}

// impl EventsubStatus {
//     pub fn as_str(&self) -> &'static str {
//         match self {
//             EventsubStatus::Enabled => "enabled",
//             EventsubStatus::WebhookCallbackVerificationPending => "webhook_callback_verification_pending",
//             EventsubStatus::WebhookCallbackVerificationFailed => "webhook_callback_verification_failed",
//             EventsubStatus::NotificationFailuresExceeded => "notification_failures_exceeded",
//             EventsubStatus::AuthorizationRevoked => "authorization_revoked",
//             EventsubStatus::ModeratorRemoved => "moderator_removed",
//             EventsubStatus::UserRemoved => "user_removed",
//             EventsubStatus::VersionRemoved => "version_removed",
//         }
//     }
// }

#[derive(Deserialize, Serialize, Clone)]
pub struct EventsubCondition {
    pub broadcaster_user_id: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EventsubTransportData {
    pub method: String,
    pub callback: String,
    pub secret: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AppAccessTokenResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub token_type: String,
}

#[derive(Deserialize, Validate)]
pub struct TwitchCodePayload {
    #[validate(length(min = 28, max = 28))]
    pub code: String,
    #[serde(deserialize_with = "str_to_int")]
    pub guild_id: i64,
}

#[derive(Deserialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub refresh_token: String,
    // pub scope: String,
}

#[derive(Deserialize)]
pub struct TwitchUserResponse {
    pub data: Vec<TwitchUser>,
}

#[derive(Deserialize, Clone)]
pub struct TwitchUser {
    #[serde(deserialize_with = "str_to_int")]
    pub id: i32,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub email: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct TwitchApiErrorResponse {
    pub error: String,
    pub status: i16,
    pub message: String,
}

#[derive(Deserialize)]
pub struct TwitchAuthErrorResponse {
    pub status: u16,
    pub message: String,
}

#[derive(Deserialize)]
pub struct TwitchSubscriptionData {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub status: EventsubStatus,
    pub version: String,
    pub cost: u8,
    pub condition: EventsubConditionData,
    pub transport: EventsubTransportData,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct EventsubConditionData {
    #[serde(deserialize_with = "str_to_int")]
    pub broadcaster_user_id: i32,
}

#[derive(Deserialize)]
pub struct EventsubEventData {
    #[serde(deserialize_with = "str_to_int")]
    pub id: i32,
    #[serde(deserialize_with = "str_to_int")]
    pub broadcaster_user_id: i32,
    pub broadcaster_user_login: String,
    pub broadcaster_user_name: String,
    pub started_at: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Deserialize)]
pub struct TwitchChallengePayload {
    pub challenge: String,
    pub subscription: TwitchSubscriptionData,
}

#[derive(Deserialize)]
pub struct TwitchNotificationPayload {
    pub subscription: TwitchSubscriptionData,
    pub event: EventsubEventData,
}

#[derive(Deserialize)]
pub struct EventsubRevocationPayload {
    pub subscription: TwitchSubscriptionData,
}

#[derive(Deserialize)]
pub enum TwitchSubscriptionStatus {
    #[serde(rename = "notification")]
    Notification,
    #[serde(rename = "WebhookCallbackVerificationPending")]
    WebhookCallbackVerification,
    #[serde(rename = "revocation")]
    Revocation,
    #[serde(rename = "enabled")]
    Enabled,
}

#[derive(Deserialize, Clone)]
pub struct StreamData {
    #[serde(deserialize_with = "str_to_int")]
    pub id: i32,
    #[serde(deserialize_with = "str_to_int")]
    pub user_id: i32,
    pub user_login: String,
    pub user_name: String,
    #[serde(deserialize_with = "str_to_int")]
    pub game_id: i32,
    pub game_name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub viewer_count: i32,
    pub started_at: String,
    pub thumbnail_url: String,
    pub language: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct TwitchStreamsResponse {
    pub data: Vec<StreamData>,
}

#[derive(Deserialize)]
pub struct StatePayload {
    pub state: String,
}

impl TwitchSubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Notification => "notification",
            Self::Revocation => "revocation",
            Self::WebhookCallbackVerification => "webhook_callback_verification",
            Self::Enabled => "enabled",
        }
    }
}

impl Serialize for ErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ErrorResponse", 3)?;

        state.serialize_field("code", &self.code.as_u16())?;
        state.serialize_field("message", &self.message)?;
        state.end()
    }
}

fn str_to_int<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}
