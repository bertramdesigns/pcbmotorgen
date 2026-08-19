//! Error types for KiCad IPC API operations.

use std::fmt;

/// Error type for all KiCad IPC operations.
#[derive(Debug)]
pub enum KiCadError {
    /// Socket unreachable or connection failed.
    Connection(String),
    /// KiCad returned a non-OK status code.
    Api { code: i32, message: String },
    /// Protobuf packing/unpacking failure.
    Protocol(String),
    /// Client not connected (must call connect() first).
    NotConnected,
}

impl std::error::Error for KiCadError {}

impl fmt::Display for KiCadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KiCadError::Connection(msg) => write!(f, "KiCad connection error: {}", msg),
            KiCadError::Api { code, message } => {
                write!(f, "KiCad API error (code={}): {}", code, message)
            }
            KiCadError::Protocol(msg) => write!(f, "KiCad protocol error: {}", msg),
            KiCadError::NotConnected => write!(f, "Not connected to KiCad"),
        }
    }
}

impl From<std::io::Error> for KiCadError {
    fn from(e: std::io::Error) -> Self {
        KiCadError::Connection(e.to_string())
    }
}
