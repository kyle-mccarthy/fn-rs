use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse};

/// This will eventually be expanded to give meaningful information, right now it is just an indicator
/// if the server is accepting requests. Does not reflect the health of the endpoints
pub fn handle(_req: HttpRequest) -> HttpResponse {
    HttpResponse::build(StatusCode::OK).body("ok")
}
