//! Production NNG transport for the KiCad IPC socket.
//!
//! [`NngTransport`] uses the `nng` crate's `Req0` (request/reply) protocol
//! over an IPC socket, connecting lazily with configurable send/recv
//! timeouts.

use std::time::Duration;

use super::KicadTransport;
use crate::errors::KiCadError;

/// NNG-based transport using the `Req0` (request/reply) protocol.
///
/// The socket is lazily opened: [`connect`](KicadTransport::connect) opens it,
/// but [`send_and_recv`](KicadTransport::send_and_recv) will also auto-connect
/// if needed.
pub(crate) struct NngTransport {
    socket_path: String,
    timeout_ms: u32,
    socket: Option<nng::Socket>,
}

impl NngTransport {
    pub(crate) fn new(socket_path: String, timeout_ms: u32) -> Self {
        Self {
            socket_path,
            timeout_ms,
            socket: None,
        }
    }

    fn ensure_connected(&mut self) -> Result<(), KiCadError> {
        if self.socket.is_some() {
            return Ok(());
        }
        self.connect_impl()
    }

    fn connect_impl(&mut self) -> Result<(), KiCadError> {
        use nng::options::{Options, RecvTimeout, SendTimeout};

        let socket = nng::Socket::new(nng::Protocol::Req0).map_err(|e| {
            KiCadError::Connection(format!("failed to create NNG Req0 socket: {e}"))
        })?;

        let timeout = Duration::from_millis(self.timeout_ms as u64);
        socket
            .set_opt::<SendTimeout>(Some(timeout))
            .map_err(|e| KiCadError::Connection(format!("failed to set send timeout: {e}")))?;
        socket
            .set_opt::<RecvTimeout>(Some(timeout))
            .map_err(|e| KiCadError::Connection(format!("failed to set recv timeout: {e}")))?;

        socket
            .dial(&self.socket_path)
            .map_err(|e| {
                KiCadError::Connection(format!(
                    "failed to dial {}: {e}",
                    self.socket_path
                ))
            })?;

        self.socket = Some(socket);
        Ok(())
    }
}

impl KicadTransport for NngTransport {
    fn connect(&mut self) -> Result<(), KiCadError> {
        if self.socket.is_some() {
            return Ok(());
        }
        self.connect_impl()
    }

    fn send_and_recv(&mut self, request_bytes: &[u8]) -> Result<Vec<u8>, KiCadError> {
        self.ensure_connected()?;

        let socket = self.socket.as_ref().unwrap();

        socket
            .send(request_bytes)
            .map_err(|(_, e)| KiCadError::Connection(format!("NNG send failed: {e}")))?;

        let msg = socket
            .recv()
            .map_err(|e| KiCadError::Connection(format!("NNG recv failed: {e}")))?;

        Ok(msg.to_vec())
    }
}