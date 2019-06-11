use crate::{handler, State};

use actix_web::web::{Data, Payload};
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use actix_web::http::StatusCode;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::FunctionConfig;

use crate::AppData;

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

    #[serde(default = FunctionResponse::default_status_code())]
    status_code: u16,
}

impl FunctionResponse {
    fn new(script: String) -> FunctionResponse {
        FunctionResponse {
            script,
            body: "".to_string(),
            headers: HashMap::new(),
            status_code: 200u16,
        }
    }

    #[allow(dead_code)]
    pub fn default_status_code() -> u16 {
        200u16
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

    body: Option<String>,

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
            body: None,
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
pub(crate) fn web_handler(state: State, req: HttpRequest, payload: Option<String>) -> HttpResponse {
    // get the config from the request
    let config: Option<&FunctionConfig> = req.app_data();

    if config.is_none() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Route configuration missing script function"
        }));
    }

    let config = config.unwrap();

    // convert the HttpRequest to the FunctionRequest
    let mut func_req = FunctionRequest::from_http_request(&req);
    let func_res = FunctionResponse::new(config.handler.clone());

    if payload.is_some() {
        func_req.body = payload;
    }

    // attempt to serialize the FunctionRequest to pass to function handler
    let func_payload = FunctionPayload::new(func_req, func_res);

    let func_payload = serde_json::to_string(&func_payload);

    if func_payload.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to serialize the request"
        }));
    }

    let func_payload = func_payload.unwrap();

    let func_res = handler::handle(state, &config, func_payload.as_str());

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

                let status_code = match StatusCode::from_u16(func_res.status_code) {
                    Ok(status_code) => status_code,
                    _ => StatusCode::OK,
                };

                let mut res = HttpResponse::build(status_code);

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

pub(crate) fn post_handler(
    payload: Payload,
    state: State,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload.concat2().from_err().and_then(|b: bytes::Bytes| {
        let body: Option<String> = String::from_utf8(b.as_ref().to_owned()).ok();
        web_handler(state, req, body)
    })
}

pub(crate) fn get_handler(
    state: State,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    futures::future::ok(web_handler(state, req, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FunctionConfig;
    use actix_web::http::{Method, StatusCode};
    use actix_web::test::{self as test, TestRequest};
    use actix_web::{web, App};

    #[test]
    fn test_get_req_echo() {
        let config = FunctionConfig::new(
            String::from("GET"),
            String::from("/"),
            String::from("cat"),
            None,
        );

        let mut app =
            test::init_service(App::new().service(web::resource("/").data(config).to(get_handler)));

        let req = TestRequest::with_uri("/").method(Method::GET).to_request();
        let res = test::call_service(&mut app, req);

        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = test::read_body(res);
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        let func_payload: FunctionPayload = serde_json::from_str(&body_str).unwrap();

        assert_eq!(func_payload.res.body, "");
    }

    #[test]
    fn test_post_req_echo() {
        let config = FunctionConfig::new(
            String::from("POST"),
            String::from("/"),
            String::from("cat"),
            None,
        );

        let mut app = test::init_service(
            App::new().service(web::resource("/").data(config).to_async(post_handler)),
        );

        let req = TestRequest::with_uri("/")
            .method(Method::POST)
            .set_payload("hello world".as_bytes())
            .to_request();
        let res = test::call_service(&mut app, req);

        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = test::read_body(res);
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        let func_payload: FunctionPayload = serde_json::from_str(&body_str).unwrap();

        assert_eq!(func_payload.req.method, "POST");

        assert_eq!(func_payload.req.body.unwrap(), String::from("hello world"));
    }
}
