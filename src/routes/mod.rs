use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

pub use notifications::init_twitch_routes;
pub use twitch::auth::init_auth_routes;
pub use twitch::service::init_service_routes;

use crate::errors::Error;
use crate::structs::ErrorResponse;

mod notifications;
mod twitch;

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Conflict => StatusCode::CONFLICT,
            Error::Twitch(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Error::BadRequest(e) => HttpResponse::BadRequest().json(ErrorResponse {
                code: StatusCode::BAD_REQUEST,
                message: e.to_string(),
            }),
            Error::Conflict => HttpResponse::BadRequest().json(ErrorResponse {
                code: StatusCode::CONFLICT,
                message: self.to_string(),
            }),
            Error::Twitch(_) => HttpResponse::BadRequest().json(ErrorResponse {
                code: StatusCode::CONFLICT,
                message: self.to_string(),
            }),
            _ => HttpResponse::InternalServerError().json(ErrorResponse {
                code: self.status_code(),
                message: self.to_string(),
            }),
        }
    }
}
