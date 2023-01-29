use actix_web::{delete, HttpResponse, post, web};
use actix_web::web::Query;
use sqlx::Row;
use crate::errors::Error;

use crate::routes::structs::TwitchCodePayload;
use crate::structs::{AppState, Result};

#[post("/")]
async fn create_notification(state: web::Data<AppState>, payload: Query<TwitchCodePayload>) -> Result<HttpResponse> {
    let token = state.fetch_user_token(payload.code.as_str()).await?;
    let user = state.fetch_user(token.as_str()).await?;

    let mut transaction = state.db.begin().await?;

    let res = sqlx::query(
        "INSERT INTO twitch_users (id, username, avatar, eventsub_id) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET username = $2, avatar = $3 RETURNING eventsub_id"
    )
        .bind(user.id)
        .bind(user.display_name.as_str())
        .bind(user.profile_image_url.as_str())
        .fetch_one(&mut transaction)
        .await?;

    let id = match res.try_get::<Option<String>, &str>("eventsub").unwrap() {
        Some(id) => id,
        None => {
            state.register_eventsub(&token, user.id).await?
        }
    };

    sqlx::query("INSERT INTO twitch_notifications (guild_id, user_id) VALUES ($1, $2)")
        .bind(payload.guild_id)
        .bind(user.id)
        .execute(&mut transaction)
        .await?;

    transaction.commit().await?;

    Ok(
        HttpResponse::Ok().body(id)
    )
}

#[delete("{id}")]
async fn delete_notification(state: web::Data<AppState>, query: web::Path<i32>) -> Result<HttpResponse> {
    let res = sqlx::query(
        "SELECT tu.eventsub_id, tn.user_id FROM twitch_users tu INNER JOIN twitch_notifications tn on tu.id = tn.user_id WHERE tn.id = $1"
    )
        .bind(query.into_inner())
        .fetch_one(&state.db)
        .await?;

    if res.is_empty() {
        return Err(Error::BadRequest("Could not find twitch notification.".to_string()));
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
async fn delete_guild_notifications(state: web::Data<AppState>, query: web::Path<i64>) -> Result<HttpResponse> {
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
        let eventsub_id =  row.get::<String, &str>("eventsub_id");
        state.delete_eventsub(eventsub_id.as_str()).await?;
    }

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn init_service_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(
            web::scope("service/twitch/notifications")
                .service(create_notification)
                .service(delete_notification)
                .service(delete_guild_notifications)
        );
}