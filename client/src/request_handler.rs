use crate::handler;

use actix_web::http::StatusCode;
use actix_web::web::Payload;
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::FunctionConfig;

/// This struct will be serialized an passed to the function
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionPayload<'a> {
    req: FunctionRequest<'a>,
    res: FunctionResponse,
}

impl<'a> FunctionPayload<'a> {
    fn new(req: FunctionRequest<'a>, res: FunctionResponse) -> FunctionPayload {
        FunctionPayload { req, res }
    }
}

/// If the function returns this struct, it will be used when sending the response
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResponse {
    script: String,
    body: String,
    headers: HashMap<String, String>,
}

impl FunctionResponse {
    fn new(script: String) -> FunctionResponse {
        FunctionResponse {
            script,
            body: "".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Response used f there was an issue with running the function, such as an internal error
/// when invoking it or an error the function encountered while running
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionError {
    config: FunctionConfig,
    error: String,
}

// Information from the HTTP Request that is forwarded to the function
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionRequest<'a> {
    path: String,
    method: String,
    headers: HashMap<String, String>,
    query_string: String,

    #[serde(skip, default)]
    inner: Option<&'a HttpRequest>,
}

impl<'a> FunctionRequest<'a> {
    fn from_http_request(req: &'a HttpRequest) -> FunctionRequest<'a> {
        FunctionRequest {
            path: req.path().to_string(),
            method: req.method().as_str().to_string(),
            headers: HashMap::new(),
            query_string: req.query_string().to_string(),
            inner: Some(req),
        }
    }
}

/// Handles an incoming response and forwards it to the function.
///
/// When the function responds, we first check for errors in stderr (error occurring during function runtime)
/// and the error prop (error that occurs during function invocation).
///
/// Next, if there are no errors we try to convert stdout's bytes to a string, if this fails an
/// internal server error is sent, ending the request.
///
/// Next, if we were able to convert to a string, we try to convert the string into a Function Response.
///
/// If this is done we use that data (headers, body, etc..) to send the request.
///
/// If is not successful we just send the string w/o setting any special headers.
///
pub(crate) fn web_handler(req: HttpRequest) -> HttpResponse {
    // get the config from the request
    let config: Option<&FunctionConfig> = req.app_data();

    if config.is_none() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Route configuration missing script function"
        }));
    }

    let config = config.unwrap();

    // convert the HttpRequest to the FunctionRequest
    let func_req = FunctionRequest::from_http_request(&req);
    let func_res = FunctionResponse::new(config.handler.clone());

    // attempt to serialize the FunctionRequest to pass to function handler
    let func_payload = FunctionPayload::new(func_req, func_res);

    let func_payload = serde_json::to_string(&func_payload);

    if func_payload.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to serialize the request"
        }));
    }

    let func_payload = func_payload.unwrap();

    let func_res = handler::handle(&config, func_payload.as_str());

    // match the response of the function and send the response
    match (
        func_res.error,
        func_res.stderr,
        func_res.stdout,
        func_res.config,
    ) {
        (None, None, Some(stdout), config) => match String::from_utf8(stdout) {
            Ok(data) => {
                let func_res = serde_json::from_str::<FunctionResponse>(&data);

                if func_res.is_err() {
                    return HttpResponse::Ok().body(data);
                }

                let func_res = func_res.unwrap();

                let mut res = HttpResponse::Ok();

                if func_res.headers.len() > 0 {
                    func_res.headers.iter().for_each(|(k, v)| {
                        res.header(k.as_str(), v.as_str());
                    });
                }

                res.body(func_res.body)
            }
            _ => {
                let err_str = "Failed to convert bytes to string".to_string();

                HttpResponse::InternalServerError().json(FunctionError {
                    config: config.clone(),
                    error: err_str,
                })
            }
        },
        // else error
        (Some(err), None, None, config) => {
            let mut http_res = HttpResponse::InternalServerError();

            http_res.json(FunctionError {
                config: config.clone(),
                error: format!("{}", &err),
            })
        }
        (None, Some(stderr), None, config) => {
            let mut http_res = HttpResponse::InternalServerError();

            let err_str = match String::from_utf8(stderr) {
                Ok(err_str) => err_str,
                _ => "Failed to convert to bytes to string".to_string(),
            };

            http_res.json(FunctionError {
                config: config.clone(),
                error: err_str,
            })
        }
        _ => HttpResponse::NotImplemented().finish(),
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
