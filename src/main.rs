use std::env;
use std::sync::Mutex;

use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlers, Logger};
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use awc::Client;
use lazy_static::lazy_static;
use log::info;
use sqlx::postgres::PgPoolOptions;

use crate::routes::{init_auth_routes, init_service_routes, init_twitch_routes};
use crate::structs::{AppState, TwitchAccessToken, TwitchState};

mod error_handler;
mod errors;
mod routes;
mod structs;
mod utils;

lazy_static! {
    static ref DB_CONNECTION_STRING: String =
        env::var("POSTGRES_DSN").expect("POSTGRES_DSN is not set but required");
    static ref CLIENT_SECRET: String =
        env::var("TWITCH_CLIENT_SECRET").expect("TWITCH_CLIENT_SECRET is not set but required");
    static ref CLIENT_ID: String =
        env::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID is not set but required");
    static ref EVENTSUB_SECRET: String =
        env::var("TWITCH_EVENTSUB_SECRET").expect("TWITCH_EVENTSUB_SECRET is not set but required");
    static ref CALLBACK_URL: String =
        env::var("TWITCH_CALLBACK_URL").expect("TWITCH_CALLBACK_URL is not set but required");
    static ref REDIRECT_URL: String =
        env::var("TWITCH_REDIRECT_URL").expect("TWITCH_REDIRECT_URL is not set but required");
    static ref BOT_URL: String = env::var("BOT_URL").expect("BOT_URL is not set but required");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DB_CONNECTION_STRING.as_str())
        .await
        .expect("Error building a connection pool");

    let env = env_logger::Env::default().default_filter_or("INFO").default_write_style_or("always");
    env_logger::init_from_env(env);

    info!("Starting webserver...");

    HttpServer::new(move || {
        let client = Client::new();

        App::new()
            .app_data(Data::new(AppState {
                db: pool.clone(),
                twitch: TwitchState {
                    client_secret: CLIENT_SECRET.as_str(),
                    client_id: CLIENT_ID.as_str(),
                    redirect_url: REDIRECT_URL.as_str(),
                    callback_url: CALLBACK_URL.as_str(),
                    eventsub_secret: EVENTSUB_SECRET.as_str(),
                    app_token: Mutex::new(TwitchAccessToken {
                        access_token: String::from(""),
                        expires_at: 0u64,
                    }),
                },
                client,
                bot_url: BOT_URL.as_str(),
            }))
            .wrap(Logger::default())
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::NOT_FOUND, error_handler::not_found_handler),
            )
            .configure(init_service_routes)
            .configure(init_twitch_routes)
            .configure(init_auth_routes)
    })
    .bind(("0.0.0.0", 3000))?
    .workers(2)
    .run()
    .await
}
