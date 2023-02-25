use actix_web::{delete, post, web, HttpResponse};
use sqlx::Row;

use crate::errors::Error;
use crate::structs::{AppState, Result};

use super::structs::TwitchCodePayload;

#[post("")]
async fn create_notification(
    state: web::Data<AppState>,
    payload: web::Json<TwitchCodePayload>,
) -> Result<HttpResponse> {
    let token = state.fetch_user_token(payload.code.as_str()).await?;
    let user = state.fetch_user(token.as_str()).await?;

    let mut transaction = state.db.begin().await?;

    let pg_res = sqlx::query(
        "INSERT INTO twitch_users (id, username, avatar, eventsub_id) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET username = $2, avatar = $3 RETURNING eventsub_id"
    )
        .bind(user.id)
        .bind(user.display_name.as_str())
        .bind(user.profile_image_url.as_str())
        .fetch_one(&mut transaction)
        .await?;

    if pg_res
        .try_get::<Option<String>, &str>("eventsub")
        .unwrap()
        .is_none()
    {
        state.register_eventsub(&token, user.id).await?;
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

#[delete("{id}")]
async fn delete_notification(
    state: web::Data<AppState>,
    query: web::Path<i32>,
) -> Result<HttpResponse> {
    let res = sqlx::query(
        "SELECT tu.eventsub_id, tn.user_id FROM twitch_users tu INNER JOIN twitch_notifications tn on tu.id = tn.user_id WHERE tn.id = $1"
    )
        .bind(query.into_inner())
        .fetch_one(&state.db)
        .await?;

    if res.is_empty() {
        return Err(Error::BadRequest(
            "Could not find twitch notification.".to_string(),
        ));
    }

    let eventsub_id = res.get::<String, &str>("eventsub_id");
    let user_id = res.get::<i32, &str>("user_id");

    state.delete_eventsub(eventsub_id.as_str()).await?;

    sqlx::query("DELETE FROM twitch_users WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

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

    // delete all unused eventsubs
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
