//! `ApiError` — frontend-side IPC error type.
//!
//! Variants mirror `vedge_tauri::error::CommandError` 1:1 (deserialized
//! from the `{kind, message}` envelope that
//! `#[serde(tag = "kind", content = "message")]` produces). Three extra
//! variants cover local-only failures: serialization / deserialization
//! of the IPC value, and unexpected promise-rejection payloads that
//! don't match the known envelope shape.
//!
//! Usage pattern in components:
//!
//! ```ignore
//! match api::vault::unlock(&input).await {
//!     Ok(()) => nav("/v", Default::default()),
//!     Err(ApiError::WrongCredentials) => set_error.set(Some("Wrong password.")),
//!     Err(e) => tracing::error!(?e, "unlock failed"),
//! }
//! ```

use thiserror::Error;
use wasm_bindgen::JsValue;

use vedge_ipc::envelope::{self, ErrorEnvelope};

#[derive(Debug, Error)]
pub enum ApiError {
    // ---- shell-reported command failures ----
    #[error("wrong password or Secret Key")]
    WrongCredentials,
    #[error("decryption failed")]
    Decryption,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("vault is not open: {0}")]
    VaultNotOpen(String),
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("biometric error: {0}")]
    Biometric(String),
    #[error("storage failure: {0}")]
    Storage(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("a vault already exists at this location")]
    AlreadyExists,
    #[error("internal error")]
    Internal,

    // ---- local-only ----
    #[error("failed to serialize command args: {0}")]
    Serialize(String),
    #[error("failed to deserialize command result: {0}")]
    Deserialize(String),
    /// Raised when the invoke promise rejects with a payload that doesn't
    /// match the expected `{kind, message}` envelope (e.g. Tauri runtime
    /// failure outside the command body).
    #[error("IPC transport error: {0}")]
    Transport(String),
}

impl ApiError {
    pub(crate) fn serialize<E: core::fmt::Display>(e: E) -> Self {
        Self::Serialize(e.to_string())
    }

    pub(crate) fn deserialize<E: core::fmt::Display>(e: E) -> Self {
        Self::Deserialize(e.to_string())
    }

    /// Decode the JS rejection value into an `ApiError`. Unknown shapes
    /// fall through to `Transport(format!("{raw:?}"))` so the caller can
    /// still log something useful.
    pub(crate) fn from_rejection(raw: JsValue) -> Self {
        match serde_wasm_bindgen::from_value::<ErrorEnvelope>(raw.clone()) {
            Ok(env) => Self::from_envelope(env),
            Err(_) => Self::Transport(format!("{raw:?}")),
        }
    }

    fn from_envelope(env: ErrorEnvelope) -> Self {
        let msg = env.message.unwrap_or_default();
        match env.kind.as_str() {
            envelope::kind::WRONG_CREDENTIALS => Self::WrongCredentials,
            envelope::kind::DECRYPTION => Self::Decryption,
            envelope::kind::NOT_FOUND => Self::NotFound(msg),
            envelope::kind::VAULT_NOT_OPEN => Self::VaultNotOpen(msg),
            envelope::kind::KEYCHAIN => Self::Keychain(msg),
            envelope::kind::BIOMETRIC => Self::Biometric(msg),
            envelope::kind::STORAGE => Self::Storage(msg),
            envelope::kind::INVALID => Self::Invalid(msg),
            envelope::kind::ALREADY_EXISTS => Self::AlreadyExists,
            envelope::kind::INTERNAL => Self::Internal,
            other => Self::Transport(format!("unknown error kind `{other}`: {msg}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;
    use vedge_ipc::envelope::ErrorEnvelope;

    fn env(kind: &str, message: Option<&str>) -> ErrorEnvelope {
        ErrorEnvelope {
            kind: kind.to_string(),
            message: message.map(str::to_string),
        }
    }

    #[test]
    fn maps_already_exists() {
        assert!(matches!(
            ApiError::from_envelope(env("AlreadyExists", None)),
            ApiError::AlreadyExists
        ));
    }

    #[test]
    fn maps_wrong_credentials() {
        assert!(matches!(
            ApiError::from_envelope(env("WrongCredentials", None)),
            ApiError::WrongCredentials
        ));
    }

    #[test]
    fn maps_not_found_with_message() {
        match ApiError::from_envelope(env("NotFound", Some("entry x"))) {
            ApiError::NotFound(m) => assert_eq!(m, "entry x"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn maps_biometric_with_message() {
        match ApiError::from_envelope(env("Biometric", Some("no biometric enrollment"))) {
            ApiError::Biometric(m) => assert_eq!(m, "no biometric enrollment"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn unknown_kind_falls_through_to_transport() {
        assert!(matches!(
            ApiError::from_envelope(env("Bogus", None)),
            ApiError::Transport(_)
        ));
    }
}
