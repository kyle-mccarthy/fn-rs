use crate::config::FunctionConfig;
use crate::handler::HandlerError;
use crate::socket::{Socket, SocketError};

/// The task contains information about what was passed to the function and what the function responded with.
#[derive(Debug)]
pub struct Task<'a> {
    /// The location of the "function" - an executable that will be invoked
    pub config: &'a FunctionConfig,

    /// Data passed to the function
    pub stdin: Vec<u8>,

    /// Data returned by the function
    pub stdout: Option<Vec<u8>>,

    /// Information about an error that occurred during the functions runtime
    pub stderr: Option<Vec<u8>>,

    /// Error that occurred while invoking the function
    pub error: Option<HandlerError>,
}

impl<'a> Task<'a> {
    pub fn new(config: &'a FunctionConfig, stdin: Vec<u8>) -> Task {
        Task {
            config,
            stdin,
            stdout: None,
            stderr: None,
            error: None,
        }
    }

    pub fn exec(&mut self, mut socket: Socket) -> Result<(), HandlerError> {
        socket.connect()?;

        socket.poll_write(2500)?;

        socket.write(&self.stdin)?;

        socket.poll_read(60 * 1000)?;

        let (_, buf) = socket.read_all()?;

        self.stdout = Some(buf);

        socket.close()?;

        Ok(())
    }
}
