use crate::core::config::FunctionConfig;
use crate::unix_socket::socket::{Socket, SocketError};

use crate::core::state::{HandleMap, State};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Child;

use crate::core::request_handler::FunctionPayload;
use failure::Compat;
use nix::sys::socket::SockAddr;
use r2d2::{Pool, PooledConnection};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tempfile::TempDir;

#[derive(Debug, Fail)]
pub enum HandlerError {
    #[fail(display = "Failed to create the temporary file")]
    TempfileError(std::io::Error),

    #[fail(display = "Failed to create the socket")]
    CreationError,

    #[fail(display = "Failed to create the socket address")]
    AddressError,

    #[fail(display = "IO Error {}", _0)]
    IOError(&'static str, std::io::Error),

    #[fail(display = "Stdin Error {}", _0)]
    StdinError(&'static str),

    #[fail(display = "Subproc err")]
    ProcessError(std::io::Error),

    #[fail(display = "Failed to communicate with the subprocess")]
    CommunicationError(nix::Error),

    #[fail(display = "Failed to create the socket {}", _0)]
    SocketError(SocketError),

    #[fail(display = "Failed to kill child process")]
    KillError(nix::Error),

    #[fail(display = "Failed to lock on handles")]
    LockError,

    #[fail(display = "Failed to insert")]
    HashInsertionError,

    #[fail(display = "Failed to get mut from hash")]
    HashGetError,

    #[fail(display = "Failed to get writable handle from hash")]
    HashMutableHandleError,

    #[fail(display = "Failed to creat pool of sockets")]
    PoolError,

    #[fail(display = "Failed to get pooled connection")]
    PooledConnectionError,
}

impl From<SocketError> for HandlerError {
    fn from(socket_error: SocketError) -> HandlerError {
        HandlerError::SocketError(socket_error)
    }
}

/*pub fn exec_task<'a, 'b: 'a>(data: State, task: &'a mut Task<'b>) -> Result<(), HandlerError> {
    Handle::find_or_create_with_socket(data, task, |mut socket, task| task.exec(&mut socket))
}*/

/// Returns a task which contains information about the task including the output
///
/// # Arguments
/// * `config` - Configuration defining the behavior of the function
/// * `incoming` - The incoming data passed to the executable - serialized FunctionPayload
///
/// If the stdout can be deserialized to the FunctionResponse (passed in via the FunctionPayload)
/// we will attempt to set headers and send the body. This allows for custom content types to be sent.
///
//pub fn handle<'a>(data: State, config: &'a FunctionConfig, incoming: &str) -> Task<'a> {
//    let mut task = Task::new(config, incoming.as_bytes().to_vec());
//    let output = exec_task(data, &mut task);
//
//    if let Some(stdout) = &task.stdout {
//        if stdout.len() == 0 {
//            task.stdout = None;
//        }
//    }
//
//    if let Some(stderr) = &task.stderr {
//        if stderr.len() == 0 {
//            task.stderr = None;
//        }
//    }
//
//    if let Err(err) = output {
//        task.error = Some(err);
//    }
//
//    task
//}

#[derive(Debug)]
pub struct Handle {
    config: FunctionConfig,
    process: Option<Child>,
    _tempdir: TempDir,
    sock_name: PathBuf,
    sock_addr: SockAddr,
}

impl Handle {
    pub fn new(config: FunctionConfig) -> Result<Handle, HandlerError> {
        let tempdir = tempfile::tempdir().map_err(|e| HandlerError::TempfileError(e))?;
        let name = tempdir.path().join("sock");

        let sock_name = tempdir.path().join("sock");
        let sock_addr = SockAddr::new_unix(&name).map_err(|e| HandlerError::AddressError)?;

        Ok(Handle {
            config,
            _tempdir: tempdir,
            sock_name,
            sock_addr,
            process: None,
        })
    }

    pub fn start(&mut self) -> Result<(), HandlerError> {
        let mut command = self.config.cmd();

        command.arg(self.sock_name());

        let process = command.spawn().map_err(|e| HandlerError::ProcessError(e))?;

        self.process = Some(process);

        Ok(())
    }

    pub fn sock_name(&self) -> &PathBuf {
        &self.sock_name
    }

    pub fn sock_addr(&self) -> &SockAddr {
        &self.sock_addr
    }

    pub fn make_socket(&self) -> Result<Socket, SocketError> {
        Socket::new(self.sock_addr.clone())
    }

    pub fn stop(&mut self) -> Result<(), HandlerError> {
        match &mut self.process {
            Some(process) => kill(Pid::from_raw(process.id() as i32), Signal::SIGTERM)
                .map_err(|e| HandlerError::KillError(e))?,
            _ => {}
        }

        Ok(())
    }

    pub fn find_or_create_with_socket<'a, 'b: 'a, F>(
        data: State,
        task: &'a mut FunctionPayload,
        cb: F,
    ) -> Result<(), HandlerError>
    where
        F: Fn(PooledConnection<Handle>, &mut FunctionPayload) -> Result<(), HandlerError>,
    {
        let handles_read = data.handles.read().map_err(|_| HandlerError::LockError)?;
        let contains_key = handles_read.contains_key(task.config.id());

        // need to drop immediately or following writes won't acquire lock
        drop(handles_read);

        if !contains_key {
            let mut handles_write = data.handles.write().map_err(|_| HandlerError::LockError)?;

            let mut handle = Handle::new(task.config.clone())?;

            handle.start()?;

            let pool = Pool::builder()
                //                .max_size(32)
                .build(handle)
                .map_err(|_| HandlerError::PoolError)?;

            handles_write.insert(task.config.id.clone(), Arc::new(RwLock::new(pool)));

            drop(handles_write);

            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        let handles = data.handles.read().map_err(|_| HandlerError::LockError)?;

        let handle = handles
            .get(task.config.id())
            .ok_or(HandlerError::HashMutableHandleError)?;

        let handle = handle.write().map_err(|_| HandlerError::LockError)?;

        let socket = handle
            .get()
            .map_err(|_| HandlerError::PooledConnectionError)?;

        cb(socket, task)
    }
}
