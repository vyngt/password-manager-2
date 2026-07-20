//! Shared error type for fallible IPC helpers (timestamp parse, base64
//! decode). The shell maps this to `CommandError`; the frontend maps it
//! into `ApiError::Invalid`.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IpcError {
    #[error("bad timestamp: {0}")]
    BadTimestamp(String),

    #[error("bad base64: {0}")]
    BadBase64(String),

    #[error("base64 payload must decode to {expected} bytes (got {actual})")]
    WrongLength { expected: usize, actual: usize },
}
