use fn_api::{ConvertFunction, FunctionContext, WasmResponse};

#[allow(dead_code)]
extern "C" {
    fn print(ptr: i32, len: i32);
}

#[no_mangle]
pub fn handle_request(ptr: *mut i32, len: i32) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };

    let ctx: FunctionContext = FunctionContext::from_slice(&slice).unwrap();

    let mut res = ctx.res;

    res.body = "hello from wasm -- test".to_string();

    res.headers.insert("x-test".to_string(), "abc".to_string());

    let res_bytes = res.to_bytes().unwrap();

    //    let debug = format!("res_bytes (wasm) : {}", res_bytes.len());
    //    unsafe {
    //        print(debug.as_ptr() as i32, debug.len() as i32);
    //    }

    let wasm_response = WasmResponse::new(res_bytes.as_ptr() as i32, res_bytes.len() as i32);

    let bytes = wasm_response.to_bytes().unwrap();

    //    let debug = format!("wasm_res_bytes (wasm) : {}", bytes.len());
    //    unsafe {
    //        print(debug.as_ptr() as i32, debug.len() as i32);
    //    }

    bytes.as_ptr() as i32
}
