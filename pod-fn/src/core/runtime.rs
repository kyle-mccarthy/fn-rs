use crate::core::config::FunctionConfig;
use crate::core::request_handler::{FunctionPayload, FunctionResponse};
use crate::core::state::AppData;
use failure::{Error, Fail};
use parking_lot::RwLock;
use std::sync::Arc;

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

struct Runtime {
    config: FunctionConfig,
    inner: RuntimeManager,
}

/// A runtime can be defined to allow for different approaches to function invocation
pub trait RuntimeManager {
    fn find_or_initialize(
        data: AppData,
        config: &FunctionConfig,
    ) -> Result<Arc<RwLock<dyn RuntimeManager>>, failure::Error>
    where
        Self: Sized + 'static,
    {
        let handles_read = data.handles.read();

        let contains_key = handles_read.contains_key(config.id());

        drop(handles_read);

        if !contains_key {
            let mut handles_write = data.handles.write();
            let runtime = Self::initialize(&config)?;

            handles_write.insert(config.id().clone(), runtime);

            drop(handles_write);
        }

        let handles_read = data.handles.read();

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
