use crate::core::config::FunctionConfig;
use crate::core::request_handler::FunctionPayload;
use crate::core::request_handler::FunctionResponse;
use crate::core::runtime::RuntimeManager;
use crate::unix_socket::socket::{Socket, SocketError};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Child;

use failure::Fail;
use nix::sys::socket::SockAddr;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Debug, Fail)]
pub enum UnixSocketError {
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

pub struct UnixSocketRuntime {
    config: FunctionConfig,
    process: Option<Child>,
    _tempdir: TempDir,
    sock_name: PathBuf,
    sock_addr: SockAddr,
}

impl RuntimeManager for UnixSocketRuntime {
    fn initialize(
        config: &FunctionConfig,
    ) -> Result<Arc<RwLock<UnixSocketRuntime>>, failure::Error> {
        let tempdir = tempfile::tempdir().map_err(|e| UnixSocketError::TempfileError(e))?;

        let name = tempdir.path().join("sock");

        let sock_name = tempdir.path().join("sock");
        let sock_addr = SockAddr::new_unix(&name).map_err(|_| UnixSocketError::AddressError)?;

        let mut runtime = UnixSocketRuntime {
            config: config.clone(),
            _tempdir: tempdir,
            sock_name,
            sock_addr,
            process: None,
        };

        runtime.start()?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        Ok(Arc::new(RwLock::new(runtime)))
    }

    fn shutdown(&mut self) -> Result<(), failure::Error> {
        match &mut self.process {
            Some(process) => kill(Pid::from_raw(process.id() as i32), Signal::SIGTERM)
                .map_err(|e| UnixSocketError::KillError(e))?,
            _ => {}
        }

        Ok(())
    }

    fn handle_request(
        &self,
        mut payload: FunctionPayload,
    ) -> Result<FunctionResponse, failure::Error> {
        let str_payload = serde_json::to_string(&payload)?;

        let mut socket = self.make_socket()?;

        socket.connect()?;

        socket.poll_write(2500)?;

        socket.write(str_payload.as_bytes())?;

        socket.poll_read(2500)?;

        let (_, buf) = socket.read_all()?;

        payload.res.body = String::from_utf8(buf.to_vec())?;

        socket.close()?;

        Ok(payload.res)
    }
}

impl UnixSocketRuntime {
    pub fn start(&mut self) -> Result<(), UnixSocketError> {
        let mut command = self.config.cmd();

        command.arg(self.sock_name());

        let process = command
            .spawn()
            .map_err(|e| UnixSocketError::ProcessError(e))?;

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
}
