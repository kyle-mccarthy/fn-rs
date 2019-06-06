use crate::handler;

use actix_web::http::StatusCode;
use actix_web::web::Payload;
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionRequest<'a> {
    path: &'a str,
    method: &'a str,
    headers: HashMap<&'a str, &'a str>,
    query_string: &'a str,

    #[serde(skip, default)]
    inner: Option<&'a HttpRequest>,
}

impl<'a> FunctionRequest<'a> {
    fn from_http_request(req: &'a HttpRequest) -> FunctionRequest<'a> {
        FunctionRequest {
            path: req.path(),
            method: req.method().as_str(),
            headers: HashMap::new(),
            query_string: req.query_string(),
            inner: Some(req),
        }
    }
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
    // get the config from the request
    let config: Option<&FuncConfig> = req.app_data();

    if config.is_none() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Route configuration missing script function"
        }));
    }

    let config = config.unwrap();

    // @todo do something with the config

    // convert the HttpRequest to the FunctionRequest
    let func_req = FunctionRequest::from_http_request(&req);

    // attempt to serialize the FunctionRequest to pass to function handler
    let func_req = serde_json::to_string(&func_req);

    if func_req.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to serialize the request"
        }));
    }

    let func_req = func_req.unwrap();

    let func_res = handler::handle(config.script.as_str(), func_req.as_str());

    // match the response of the function and send the response
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
        // else error
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
