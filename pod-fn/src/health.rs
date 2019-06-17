use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse};

pub fn handle(_req: HttpRequest) -> HttpResponse {
    HttpResponse::build(StatusCode::OK).body("ok")
}
