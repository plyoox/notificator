use actix_web::{get, web};

use crate::routes::twitch::structs::StatePayload;
use crate::structs::AppState;

#[get("/")]
async fn login_url(state: web::Data<AppState>, req_state: web::Query<StatePayload>) -> String {
    format!(
        "https://id.twitch.tv/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope=user:read:email&state={}",
        state.twitch.client_id,
        state.twitch.redirect_url,
        req_state.into_inner().state.as_str()
    )
}

pub fn init_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("service/twitch/auth").service(login_url));
}
