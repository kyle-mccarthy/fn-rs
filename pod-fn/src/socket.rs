use nix::poll::{poll, PollFd, PollFlags};
use nix::sys::socket::{recv, send, MsgFlags};
use nix::sys::socket::{AddressFamily, SockAddr, SockFlag, SockType};
use nix::unistd::{close, read, write};
use std::os::unix::io::RawFd;

use crate::handler::Handle;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug, Fail)]
pub enum SocketError {
    #[fail(display = "Failed to create the temporary file")]
    TempfileError(std::io::Error),

    #[fail(display = "Failed to create the socket")]
    CreationError,

    #[fail(display = "Failed to create the socket address")]
    AddressError,

    #[fail(display = "Socket failed to bind to address {}", _0)]
    BindError(nix::Error),

    #[fail(display = "Socket failed to listen {}", _0)]
    ListenError(nix::Error),

    #[fail(display = "Socket failed to accept connections {}", _0)]
    AcceptError(nix::Error),

    #[fail(display = "Socket failed to send data {}", _0)]
    SendError(nix::Error),

    #[fail(display = "Socket failed to read data {}", _0)]
    ReadError(nix::Error),

    #[fail(display = "Socket failed to recv data {}", _0)]
    RecvError(nix::Error),

    #[fail(display = "Socket failed to write data {}", _0)]
    WriteError(nix::Error),

    #[fail(display = "Failed to connect to socket {}", _0)]
    ConnectError(nix::Error),

    #[fail(display = "Failed to close the socket {}", _0)]
    CloseError(nix::Error),

    #[fail(display = "Failed to become ready in time {}", _0)]
    PollTimeout(nix::Error),

    #[fail(display = "Bad file number (is none?)")]
    BadFileNumber,
}

#[derive(Debug)]
pub struct Socket<'a> {
    fd: RawFd,
    handle: &'a Handle,
}

impl<'a> Socket<'a> {
    pub fn new(handle: &'a Handle) -> Result<Socket, SocketError> {
        let fd = nix::sys::socket::socket(
            AddressFamily::Unix,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(|_| SocketError::CreationError)?;

        Ok(Socket { fd, handle })
    }

    pub fn bind(&self) -> Result<(), SocketError> {
        nix::sys::socket::bind(self.fd(), &self.handle.sock_addr())
            .map_err(|e| SocketError::BindError(e))
    }

    pub fn listen(&self) -> Result<(), SocketError> {
        nix::sys::socket::listen(self.fd(), 10).map_err(|e| SocketError::ListenError(e))
    }

    pub fn accept(&self) -> Result<RawFd, SocketError> {
        nix::sys::socket::accept(self.fd()).map_err(|e| SocketError::AcceptError(e))
    }

    pub fn connect(&self) -> Result<(), SocketError> {
        nix::sys::socket::connect(self.fd(), &self.handle.sock_addr())
            .map_err(|e| SocketError::ConnectError(e))
    }

    pub fn close(&mut self) -> Result<(), SocketError> {
        close(self.fd()).map_err(|e| SocketError::CloseError(e))
    }

    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, SocketError> {
        send(self.fd(), buf, MsgFlags::empty()).map_err(|e| SocketError::SendError(e))
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, SocketError> {
        write(self.fd(), buf).map_err(|e| SocketError::WriteError(e))
    }

    pub fn recv(&self, buf: &'a mut [u8]) -> Result<(usize, &'a [u8]), SocketError> {
        let bytes_read =
            recv(self.fd(), buf, MsgFlags::empty()).map_err(|e| SocketError::RecvError(e))?;
        Ok((bytes_read, buf))
    }

    pub fn read(&self, buf: &'a mut [u8]) -> Result<(usize, &'a [u8]), SocketError> {
        let bytes_read = read(self.fd(), buf).map_err(|e| SocketError::ReadError(e))?;
        Ok((bytes_read, buf))
    }

    pub fn read_all(&self) -> Result<(usize, Vec<u8>), SocketError> {
        let buffer_size = 128;
        let mut buf: Vec<u8> = vec![0; 128];
        let mut output: Vec<u8> = vec![];
        let mut total_bytes = 0;

        while {
            let (bytes_read, buf) = self.read(&mut buf)?;
            output.extend_from_slice(&buf[..bytes_read]);
            total_bytes += bytes_read;
            bytes_read > buffer_size
        } {}

        Ok((total_bytes, output))
    }

    pub fn poll_write(&mut self, timeout: i32) -> Result<i32, SocketError> {
        let poll_fd = PollFd::new(self.fd(), PollFlags::POLLOUT);
        poll(&mut [poll_fd], timeout).map_err(|e| SocketError::PollTimeout(e))
    }

    pub fn poll_read(&mut self, timeout: i32) -> Result<i32, SocketError> {
        let poll_fd = PollFd::new(self.fd(), PollFlags::POLLIN);
        poll(&mut [poll_fd], timeout).map_err(|e| SocketError::PollTimeout(e))
    }
}
