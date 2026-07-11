//! TOTP command wrappers (slice 4.2). Mirrors `vedge-tauri/src/commands/totp.rs`.

use serde::Serialize;
use vedge_ipc::{TotpCodeDto, TotpEnrolmentDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Reveal a Login's current TOTP code. Audited once per (session, entry); the
/// seed never crosses — only the ephemeral code.
pub async fn reveal_totp(vault_path: &str, entry_id: &str) -> Result<TotpCodeDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call("reveal_totp", &Args { vault_path, entry_id }).await
}

/// Parse an `otpauth://` URI or a bare Base32 secret into enrolment fields
/// (secret + params + issuer/account). The raw string already came from WASM.
pub async fn parse_totp_enrolment(raw: &str) -> Result<TotpEnrolmentDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        raw: &'a str,
    }
    call("parse_totp_enrolment", &Args { raw }).await
}
