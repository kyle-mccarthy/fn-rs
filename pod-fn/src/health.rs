use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse};

pub fn handle(req: HttpRequest) -> HttpResponse {
    HttpResponse::build(StatusCode::OK).body("ok")
}
