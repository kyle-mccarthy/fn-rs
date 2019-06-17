use actix_web::web::Payload;
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use actix_web::http::StatusCode;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::config::FunctionConfig;
use crate::core::runtime::RuntimeManager;
use crate::core::state::AppData;

use crate::unix_socket::runtime::UnixSocketRuntime;

/// This struct will be serialized an passed to the function
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionPayload<'a> {
    pub req: FunctionRequest<'a>,
    pub res: FunctionResponse,
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
    pub body: String,
    pub headers: HashMap<String, String>,

    #[serde(default = FunctionResponse::default_status_code())]
    pub status_code: u16,
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

    pub fn to_string(&self) -> Result<String, failure::Error> {
        serde_json::to_string(self).map_err(|e| e.into())
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
            body: Some("".to_string()),
            inner: Some(req),
        }
    }

    pub fn to_string(&self) -> Result<String, failure::Error> {
        serde_json::to_string(self).map_err(|e| e.into())
    }
}

fn handle_request(
    data: AppData,
    config: FunctionConfig,
    payload: FunctionPayload,
) -> Result<FunctionResponse, failure::Error> {
    if &config.runtime != "unix_socket" {
        unimplemented!("Runtime ({}) does not exist", config.runtime);
    }

    let runtime = UnixSocketRuntime::find_or_initialize(data, &config)?;
    let lock_guard = runtime.read().unwrap();

    lock_guard.handle_request(payload)
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
pub(crate) fn web_handler(
    state: AppData,
    req: HttpRequest,
    payload: Option<String>,
) -> HttpResponse {
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

    // the runtime manager is responsible for any serialization
    let func_res = handle_request(state, config.clone(), func_payload);

    match func_res {
        Ok(func_res) => {
            let status_code = match StatusCode::from_u16(func_res.status_code) {
                Ok(status_code) => status_code,
                _ => StatusCode::OK,
            };

            let mut http_res = HttpResponse::build(status_code);

            if func_res.headers.len() > 0 {
                func_res.headers.iter().for_each(|(k, v)| {
                    http_res.header(k.as_str(), v.as_str());
                });
            }

            http_res.body(func_res.body)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub(crate) fn post_handler(
    payload: Payload,
    state: AppData,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload.concat2().from_err().and_then(|b: bytes::Bytes| {
        let body: Option<String> = String::from_utf8(b.as_ref().to_owned()).ok();
        web_handler(state, req, body)
    })
}

pub(crate) fn get_handler(
    state: AppData,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    futures::future::ok(web_handler(state, req, None))
}

#[cfg(test)]
mod tests {}
