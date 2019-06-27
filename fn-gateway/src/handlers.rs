use actix_web::web::Payload;
use actix_web::{Error, HttpRequest, HttpResponse};
use futures::{Future, Stream};

use actix_web::http::StatusCode;

use fn_api::{ConvertFunction, FunctionContext, FunctionRequest, FunctionResponse};
use fn_core::config::FunctionConfig;
use fn_core::runtime::RuntimeManager;
use fn_core::state::AppData;
use fn_unix_socket_runtime::runtime::UnixSocketRuntime;
use fn_wasm_runtime::runtime::WasmRuntime;

use actix_web::dev::Body;
use bytes::Bytes;

/// Determine the runtime to use from the FunctionConfig and send the request to that runtime.
/// If the runtime has not been initialized, this will result in a cold start for the function.
fn handle_request(
    data: AppData,
    config: FunctionConfig,
    payload: FunctionContext,
) -> Result<Vec<u8>, failure::Error> {
    let runtime = match config.runtime.as_str() {
        "unix_socket" => UnixSocketRuntime::find_or_initialize(data, &config)?,
        "wasm" => WasmRuntime::find_or_initialize(data, &config)?,
        _ => {
            unimplemented!("Runtime ({}) does not exist", config.runtime);
        }
    };

    let lock_guard = runtime.read();
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
pub(crate) fn web_handler(state: AppData, req: HttpRequest, payload: Option<&str>) -> HttpResponse {
    // get the config from the request
    let config: Option<&FunctionConfig> = req.app_data();

    if config.is_none() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Route configuration missing script function"
        }));
    }

    let config = config.unwrap();

    // convert the HttpRequest to the FunctionRequest
    let mut func_req = FunctionRequest::new(
        &config.handler,
        req.path(),
        req.method().as_str(),
        req.query_string(),
    );
    let func_res = FunctionResponse::new();

    if payload.is_some() {
        func_req.body = payload;
    }

    // attempt to serialize the FunctionRequest to pass to function handler
    let func_payload = FunctionContext::new(func_req, func_res);

    // the runtime manager is responsible for any serialization
    let func_res = handle_request(state, config.clone(), func_payload);

    match func_res {
        Ok(func_res) => {
            let res = FunctionResponse::from_slice(&func_res);

            if res.is_err() {
                println!("is error {:?}", res);
                let mut res = HttpResponse::build(StatusCode::OK);
                res.set_header("content-type", "text/plain");
                return res.body(Body::Bytes(Bytes::from(func_res)));
            }

            let func_res = res.unwrap();

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

/// Handles POST request to the gateway
pub(crate) fn post_handler(
    payload: Payload,
    state: AppData,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload.concat2().from_err().and_then(|b: bytes::Bytes| {
        let body: Option<&str> = std::str::from_utf8(b.as_ref()).ok();
        web_handler(state, req, body)
    })
}

/// Handles GET request to the gateway
pub(crate) fn get_handler(
    state: AppData,
    req: HttpRequest,
) -> impl Future<Item = HttpResponse, Error = Error> {
    futures::future::ok(web_handler(state, req, None))
}
