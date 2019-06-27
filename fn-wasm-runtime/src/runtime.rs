use fn_api::{ConvertFunction, FunctionContext, WasmResponse};
use fn_core::config::FunctionConfig;
use fn_core::runtime::RuntimeManager;

use parking_lot::RwLock;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use wasmer_runtime::{func, imports, validate, Ctx};

use failure::Fail;

use wasmer_clif_backend::CraneliftCompiler;

#[derive(Debug, Fail)]
pub enum WasmRuntimeError {
    #[fail(display = "Failed to open file {}", _0)]
    FileOpenError(std::io::Error),

    #[fail(display = "Failed to read file {}", _0)]
    FileReadError(std::io::Error),

    #[fail(display = "Failed to validate wasm")]
    InvalidWasmError,

    #[fail(display = "Failed to compile wasm {}", _0)]
    CompileWasmError(wasmer_runtime::error::CompileError),

    #[fail(display = "Failed to instantiate module")]
    InstantiationError,

    #[fail(display = "Failed to resolve function in module {}", _0)]
    ResolveError(wasmer_runtime::error::ResolveError),

    #[fail(display = "Failed to serialize the data")]
    SerializeError,

    #[fail(display = "Error during runtime: {}", _0)]
    RuntimeError(String),

    #[fail(display = "Trap during runtime: {}", _0)]
    RuntimeTrap(String),
}

pub struct WasmRuntime {
    pub config: FunctionConfig,
    pub bytes: Vec<u8>,
    pub module: wasmer_runtime::Module,
}

impl WasmRuntime {
    pub fn new(
        config: FunctionConfig,
        bytes: Vec<u8>,
        module: wasmer_runtime::Module,
    ) -> WasmRuntime {
        WasmRuntime {
            config,
            bytes,
            module,
        }
    }
}

impl RuntimeManager for WasmRuntime {
    fn initialize(config: &FunctionConfig) -> Result<Arc<RwLock<Self>>, failure::Error>
    where
        Self: Sized,
    {
        let mut file =
            File::open(&config.handler).map_err(|e| WasmRuntimeError::FileOpenError(e))?;

        let mut buf = vec![];

        file.read_to_end(&mut buf)
            .map_err(|e| WasmRuntimeError::FileReadError(e))?;

        if !validate(&buf) {
            return Err(WasmRuntimeError::InvalidWasmError)?;
        }

        let compiler = &CraneliftCompiler::new();

        let module = wasmer_runtime::compile_with(&buf, compiler)
            .map_err(|e| WasmRuntimeError::CompileWasmError(e))?;

        let runtime = WasmRuntime::new(config.clone(), buf, module);

        Ok(Arc::new(RwLock::new(runtime)))
    }

    fn shutdown(&mut self) -> Result<(), failure::Error> {
        Ok(())
    }

    fn handle_request(&self, payload: FunctionContext) -> Result<Vec<u8>, failure::Error> {
        let data = payload.to_bytes()?;
        let import_object = imports! {
            "env" => {
                "print" => func!(print),
            },
        };

        let mut instance = self.module.instantiate(&import_object).map_err(|e| {
            dbg!(e);
            WasmRuntimeError::InstantiationError
        })?;

        let ctx = instance.context_mut();

        let memory = ctx.memory(0);

        for (byte, cell) in data
            .iter()
            .zip(memory.view::<u8>()[1 as usize..=data.len() as usize].iter())
        {
            cell.set(byte.to_owned());
        }

        let handle_request = instance
            .func::<(i32, i32), i32>("handle_request")
            .map_err(|e| WasmRuntimeError::ResolveError(e))?;

        // returns the ptr to the WasmResponse bytes
        let raw_ptr = handle_request
            .call(1i32, data.len() as i32)
            .map_err(|e| match e {
                wasmer_runtime::error::RuntimeError::Error { data } => {
                    dbg!(data);
                    WasmRuntimeError::RuntimeError("?".to_string())
                }
                wasmer_runtime::error::RuntimeError::Trap { msg } => {
                    WasmRuntimeError::RuntimeTrap(msg.to_string())
                }
            })?;

        // load the WasmResponse
        let end_location: usize = raw_ptr as usize + 16usize;
        let ctx = instance.context();
        let view = &ctx.memory(0).view();
        let bytes = view[raw_ptr as usize..end_location]
            .iter()
            .map(|cell| cell.get())
            .collect::<Vec<u8>>();

        let wasm_response = WasmResponse::from_slice(&bytes)?;

        // using the WasmResponse load the FunctionResponse from memory
        let end_location: usize = wasm_response.ptr as usize + wasm_response.len as usize;
        let ctx = instance.context();
        let view = &ctx.memory(0).view();
        let bytes = view[wasm_response.ptr as usize..end_location]
            .iter()
            .map(|cell| cell.get())
            .collect::<Vec<u8>>();

        Ok(bytes)
    }
}

fn print(ctx: &mut Ctx, ptr: u32, len: u32) {
    let memory = ctx.memory(0);

    dbg!(format!("println ptr {} len {}", ptr, len));

    let str_slice = &memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect::<Vec<u8>>();

    let str_utf8 = std::str::from_utf8(&str_slice);

    match str_utf8 {
        Ok(str) => println!("{}", str),
        Err(e) => {
            dbg!(e);
        }
    }
}
