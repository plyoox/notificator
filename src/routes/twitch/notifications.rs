use actix_web::{guard, HttpRequest, HttpResponse, post, web};
use actix_web::guard::GuardContext;
use awc::error::StatusCode;
use hmac::{Hmac, Mac};
use log::{error, warn};
use sha2::Sha256;

use crate::errors::Error;
use crate::structs::{AppState, ErrorResponse, Result};

use super::structs::{
    EventsubRevocationPayload, TwitchChallengePayload, TwitchNotificationPayload,
    TwitchSubscriptionStatus,
};

#[post("twitch")]
async fn handle_eventsub(
    request: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse> {
    let headers = request.headers();

    let message_id = headers
        .get("twitch-eventsub-message-id")
        .unwrap()
        .to_str()
        .unwrap();
    let message_signature = headers
        .get("twitch-eventsub-message-signature")
        .unwrap()
        .to_str()
        .unwrap();
    let message_timestamp = headers
        .get("twitch-eventsub-message-timestamp")
        .unwrap()
        .to_str()
        .unwrap();
    let message_type = headers
        .get("twitch-eventsub-message-type")
        .unwrap()
        .to_str()
        .unwrap();

    if message_signature.len() < 10 {
        error!(
            "Received invalid signature from request: {}",
            message_signature.len()
        );

        return Err(Error::BadRequest("Invalid signature received.".to_string()));
    }

    let shorted_signature = message_signature[7..message_signature.len()].to_owned();
    let body_bytes = String::from_utf8(body.to_vec());
    if body_bytes.is_err() {
        error!("Could not decode body of eventsub.");

        return Err(Error::InternalServer(
            "Could not decode body of eventsub.".to_string(),
        ));
    }

    let body_str = body_bytes.unwrap().as_str().to_string();
    let mac_message = message_id.to_string() + message_timestamp + body_str.as_str();

    let mut mac = Hmac::<Sha256>::new_from_slice(state.twitch.eventsub_secret.as_bytes()).unwrap();
    mac.update(mac_message.as_bytes());

    let decoded_signature = hex::decode(shorted_signature).unwrap();
    if mac.verify_slice(&decoded_signature).is_err() {
        return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
            code: StatusCode::UNAUTHORIZED,
            message: "Invalid signature provided.".to_string(),
        }));
    }

    if message_type == TwitchSubscriptionStatus::WebhookCallbackVerification.as_str() {
        let data = serde_json::from_str::<TwitchChallengePayload>(body_str.as_str())?;

        return Ok(HttpResponse::Ok().body(data.challenge));
    } else if message_type == TwitchSubscriptionStatus::Notification.as_str() {
        let data = serde_json::from_str::<TwitchNotificationPayload>(body_str.as_str())?;
        let stream_data = state
            .fetch_stream_data(data.event.broadcaster_user_id)
            .await?;

        let bot_url = format!(
            "{}?user_id={}&user_name={}&game_name={}&viewer_count={}&started_at={}&thumbnail_url={}&title={}",
            state.bot_url, stream_data.user_id, stream_data.user_login, stream_data.game_name.as_str(), stream_data.viewer_count,
            stream_data.started_at.as_str(), stream_data.thumbnail_url.as_str(), stream_data.title.as_str(),
        );
        let req = state.client.get(bot_url);

        actix_web::rt::spawn(async move {
            if req.send().await.is_err() {
                warn!("An error occurred while sending twitch notification to bot");
            }
        });
    } else if message_type == TwitchSubscriptionStatus::Revocation.as_str() {
        let data = serde_json::from_str::<EventsubRevocationPayload>(body_str.as_str())?;

        sqlx::query("DELETE FROM twitch_users WHERE id = $1")
            .bind(data.subscription.condition.broadcaster_user_id)
            .execute(&state.db)
            .await?;
    }

    Ok(HttpResponse::Ok().finish())
}

fn route_guard(ctx: &GuardContext) -> bool {
    let h = ctx.head().headers();

    h.contains_key("twitch-eventsub-message-id")
        && h.contains_key("twitch-eventsub-message-signature")
        && h.contains_key("twitch-eventsub-message-timestamp")
        && h.contains_key("twitch-eventsub-message-type")
}

pub fn init_twitch_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("_notify")
            .guard(guard::fn_guard(route_guard))
            .service(handle_eventsub),
    );
}
