pub use twitch::auth::init_auth_routes;
pub use notifications::init_twitch_routes;
pub use twitch::service::init_service_routes;

mod twitch;
mod notifications;
