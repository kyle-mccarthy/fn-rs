use crate::core::config::FunctionConfig;
use crate::core::request_handler::{FunctionPayload, FunctionResponse};
use crate::core::state::AppData;
use failure::{Compat, Error, Fail};
use r2d2::ManageConnection;
use std::sync::{Arc, RwLock};

#[derive(Debug, Fail)]
pub enum RuntimeError {
    #[fail(display = "failed to initialize runtime")]
    InitializationError,

    #[fail(
        display = "Encountered internal server error while handling request to function {}",
        _0
    )]
    InternalServerError(Box<Fail>),

    #[fail(display = "Encountered an error while shutting down")]
    ShutdownError(Error),

    #[fail(display = "Failed to acquire lock")]
    LockError,

    #[fail(display = "Runtime deleted from hash map while fetching")]
    RaceError,
}

// knows which "type" of runtime to use for each request based on the config

struct Runtime {
    config: FunctionConfig,
    inner: RuntimeManager,
}

pub trait RuntimeManager {
    fn find_or_initialize(
        data: AppData,
        config: &FunctionConfig,
    ) -> Result<Arc<RwLock<dyn RuntimeManager>>, failure::Error>
    where
        Self: Sized + 'static,
    {
        let handles_read = data.handles.read().map_err(|_| RuntimeError::LockError)?;

        let contains_key = handles_read.contains_key(config.id());

        drop(handles_read);

        if !contains_key {
            let mut handles_write = data.handles.write().map_err(|_| RuntimeError::LockError)?;
            let runtime = Self::initialize(&config)?;

            handles_write.insert(config.id().clone(), runtime);

            drop(handles_write);
        }

        let handles_read = data.handles.read().map_err(|_| RuntimeError::LockError)?;

        let runtime = handles_read
            .get(config.id())
            .ok_or(RuntimeError::RaceError)?;

        Ok(runtime.clone())
    }

    fn initialize(config: &FunctionConfig) -> Result<Arc<RwLock<Self>>, failure::Error>
    where
        Self: Sized;

    fn shutdown(&mut self) -> Result<(), failure::Error>;

    fn handle_request(&self, payload: FunctionPayload) -> Result<FunctionResponse, failure::Error>;
}

// request comes in
// app looks in state and looks up runtime associated with config id
// -- if runtime does not exist
