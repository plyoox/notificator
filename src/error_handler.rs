use actix_web::dev::ServiceResponse;
use actix_web::http::{header, StatusCode};
use actix_web::middleware::ErrorHandlerResponse;

use crate::structs::ErrorResponse;

pub fn not_found_handler<B>(mut res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let response_struct = ErrorResponse {
        code: StatusCode::NOT_FOUND,
        message: "Cannot find this path or this method".to_string(),
    };

    res.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));

    let (req, res) = res.into_parts();
    let res = res.set_body(serde_json::to_string(&response_struct).unwrap());

    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
