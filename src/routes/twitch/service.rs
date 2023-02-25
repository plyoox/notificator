use actix_web::{delete, post, web, HttpResponse};
use sqlx::Row;

use crate::errors::Error;
use crate::structs::{AppState, Result};

use super::structs::TwitchCodePayload;

/// # Create notification
/// Creates a notification for a specific user from the oauth authorization code
/// ## Responses
/// - 200 Successfully created notification
/// - 409 Notification already exists
/// - 500 Internal sever error
/// - 502 Twitch api error
#[post("")]
async fn create_notification(
    state: web::Data<AppState>,
    payload: web::Json<TwitchCodePayload>,
) -> Result<HttpResponse> {
    let token = state.fetch_user_token(payload.code.as_str()).await?;
    let user = state.fetch_user(token.as_str()).await?;

    let mut transaction = state.db.begin().await?;

    let pg_res = sqlx::query(
        "SELECT tn.id FROM twitch_users tu INNER JOIN twitch_notifications tn on tu.id = tn.user_id WHERE tu.id = $1 AND tn.guild_id = $2"
    )
        .bind(user.id)
        .bind(payload.guild_id)
        .fetch_optional(&mut transaction)
        .await?;

    if pg_res.is_some() {
        return Err(Error::Conflict);
    } else {
        let eventsub_id = state.register_eventsub(user.id).await?;
        // Conflict should only happen when manually deleting an user
        sqlx::query(
            "INSERT INTO twitch_users (id, username, avatar, eventsub_id) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET username = $2, avatar = $3, eventsub_id = $4",
        )
        .bind(user.id)
        .bind(user.display_name.as_str())
        .bind(user.profile_image_url.as_str())
        .bind(eventsub_id.as_str())
        .execute(&mut transaction)
        .await?;
    };

    let pg_res = sqlx::query("INSERT INTO twitch_notifications (guild_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING id")
        .bind(payload.guild_id)
        .bind(user.id)
        .fetch_one(&mut transaction)
        .await?;

    transaction.commit().await?;

    let notification_id = pg_res.get::<i32, &str>("id").to_string();
    Ok(HttpResponse::Ok().body(notification_id))
}

/// # Delete Notification
/// Deletes a notification with a specific id. If no other notifications for this user are present, the eventsub will be deleted.
/// ## Responses
/// - 204 Successfully deleted notification
/// - 400 Unknown notification
/// - 500 Internal server error
#[delete("{id}")]
async fn delete_notification(
    state: web::Data<AppState>,
    query: web::Path<i32>,
) -> Result<HttpResponse> {
    let notification_id = query.into_inner();
    let pg_res = sqlx::query("DELETE FROM twitch_notifications WHERE id = $1 RETURNING user_id")
        .bind(notification_id)
        .fetch_one(&state.db)
        .await?;

    if pg_res.is_empty() {
        return Err(Error::BadRequest("Notification not found".to_string()));
    }

    let user_id = pg_res.get::<i32, &str>("user_id");
    let res = sqlx::query(
        "SELECT tn.id FROM twitch_users tu LEFT JOIN twitch_notifications tn on tu.id = tn.user_id WHERE tu.id = $1"
    )
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    if res.get::<Option<i32>, &str>("id").is_some() {
        let eventsub_id = res.get::<String, &str>("eventsub_id");
        state.delete_eventsub(eventsub_id.as_str()).await?;

        sqlx::query("DELETE FROM twitch_users WHERE id = $1")
            .bind(user_id)
            .execute(&state.db)
            .await?;
    }

    Ok(HttpResponse::NoContent().finish())
}

/// # Delete Guild notifications
/// Deletes all notifications from a specific guild.
/// ## Responses
/// - 204 Notifications successfully deleted
/// - 500 Internal server error
#[delete("guild/{id}")]
async fn delete_guild_notifications(
    state: web::Data<AppState>,
    query: web::Path<i64>,
) -> Result<HttpResponse> {
    let mut transaction = state.db.begin().await?;

    sqlx::query("DELETE FROM twitch_notifications WHERE guild_id = $1")
        .bind(query.into_inner())
        .execute(&mut transaction)
        .await?;

    // delete all unused eventsubs, this also cleans up some lost entries
    let unused_users = sqlx::query("DELETE FROM twitch_users WHERE (SELECT count(*) FROM twitch_notifications) = 0 RETURNING eventsub_id")
        .fetch_all(&mut transaction)
        .await?;

    for row in unused_users {
        let eventsub_id = row.get::<String, &str>("eventsub_id");
        state.delete_eventsub(eventsub_id.as_str()).await?;
    }

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn init_service_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("service/twitch/notifications")
            .service(create_notification)
            .service(delete_notification)
            .service(delete_guild_notifications),
    );
}
