pub mod errors;

use crate::errors::Errors;
use failure;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

/// Trait for handling common conversions
pub trait ConvertFunction<'a>: Sized + Deserialize<'a> + Serialize {
    fn to_bytes(&self) -> Result<Vec<u8>, Errors> {
        serde_cbor::to_vec(&self).map_err(|e| Errors::SerializationError(e.to_string()))
    }

    fn from_slice(bytes: &'a [u8]) -> Result<Self, Errors> {
        serde_cbor::from_slice(&bytes).map_err(|e| Errors::DeserializationError(e.to_string()))
    }

    fn to_string(&self) -> Result<String, Errors> {
        serde_json::to_string(&self).map_err(|e| Errors::SerializationError(e.to_string()))
    }

    fn from_str(str: &'a str) -> Result<Self, Errors> {
        serde_json::from_str(&str).map_err(|e| Errors::DeserializationError(e.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionContext<'a> {
    #[serde(borrow = "'a")]
    pub req: FunctionRequest<'a>,

    pub res: FunctionResponse,
}

impl<'a> FunctionContext<'a> {
    pub fn new(req: FunctionRequest<'a>, res: FunctionResponse) -> FunctionContext<'a> {
        FunctionContext { req, res }
    }
}

impl<'a> ConvertFunction<'a> for FunctionContext<'a> {}

/// If the function returns this struct, it will be used when sending the response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionResponse {
    /// Body of the response
    pub body: String,
    /// Headers of the response
    pub headers: HashMap<String, String>,
    /// Http status code for the response, defaults to 200 (OK)
    #[serde(default = FunctionResponse::default_status_code())]
    pub status_code: u16,
}

impl FunctionResponse {
    pub fn new() -> FunctionResponse {
        FunctionResponse {
            body: String::from(""),
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

impl<'a> ConvertFunction<'a> for FunctionResponse {}

// Information from the HTTP Request that is forwarded to the function
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionRequest<'a> {
    /// The target path of the request
    pub path: &'a str,
    /// Method of the request
    pub method: &'a str,
    /// Headers of the request
    pub headers: HashMap<&'a str, &'a str>,
    /// Query string of the request, empty string if none
    pub query_string: &'a str,
    /// Body of the request
    pub body: Option<&'a str>,
    /// The location of the script for the function that handles the request
    pub script: &'a str,
}

impl<'a> FunctionRequest<'a> {
    pub fn new(
        script: &'a str,
        path: &'a str,
        method: &'a str,
        query_string: &'a str,
    ) -> FunctionRequest<'a> {
        FunctionRequest {
            script,
            path,
            method,
            headers: HashMap::new(),
            query_string,
            // @todo want to remove empty string and make it none but issue with nodejs and turbo-json-parse
            body: Some(""),
        }
    }
}

impl<'a> ConvertFunction<'a> for FunctionRequest<'a> {}

/// This is only used in the WebAssembly runtime. As of right now, only one value can be returned from a
/// function making it difficult to get both a pointer and the size of the data to read. As a helper,
/// this can be returned with a pointer to the data and with the size of the data to read.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WasmResponse {
    pub ptr: i32,
    pub len: i32,
}

impl<'a> ConvertFunction<'a> for WasmResponse {}

impl WasmResponse {
    pub fn new(ptr: i32, len: i32) -> WasmResponse {
        WasmResponse { ptr, len }
    }
}
