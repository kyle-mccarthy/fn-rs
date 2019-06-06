use crate::handler;

use actix_web::http::StatusCode;
use actix_web::web::Payload;
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use serde::{Deserialize, Serialize};

pub struct FuncConfig {
    pub script: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResponse {
    data: String,
    script: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionError {
    script: String,
    error: String,
}

#[allow(dead_code)]
pub(crate) fn async_web_handler(
    payload: Payload,
    _req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload
        .concat2()
        .from_err()
        .and_then(|_| HttpResponse::build(StatusCode::OK).body("ok"))
}

pub(crate) fn web_handler(req: HttpRequest) -> HttpResponse {
    let config: Option<&FuncConfig> = req.app_data();

    if config.is_none() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Route configuration missing script function"
        }));
    }

    let config = config.unwrap();

    let func_res = handler::handle(config.script.as_str(), "hi");

    match (
        func_res.error,
        func_res.stderr,
        func_res.stdout,
        func_res.script,
    ) {
        (_, _, Some(stdout), script) => match String::from_utf8(stdout) {
            Ok(data) => HttpResponse::Ok().json(FunctionResponse { script, data }),
            _ => {
                let err_str = "Failed to convert bytes to string".to_string();

                HttpResponse::InternalServerError().json(FunctionError {
                    script,
                    error: err_str,
                })
            }
        },
        (Some(err), None, None, script) => {
            let mut http_res = HttpResponse::InternalServerError();

            http_res.json(FunctionError {
                script,
                error: format!("{}", &err),
            })
        }
        (None, Some(stderr), None, script) => {
            let mut http_res = HttpResponse::InternalServerError();

            let err_str = match String::from_utf8(stderr) {
                Ok(err_str) => err_str,
                _ => "Failed to convert to bytes to string".to_string(),
            };

            http_res.json(FunctionError {
                script,
                error: err_str,
            })
        }
        _ => HttpResponse::NotImplemented().finish(),
    }
}
