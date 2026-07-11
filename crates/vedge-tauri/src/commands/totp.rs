//! TOTP commands (slice 4.2): an audited reveal + a stateless enrolment parser.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion of
//! `#[tauri::command(rename_all = "snake_case")]` trips `unreachable` /
//! `let_underscore_must_use`; holding the session mutex trips
//! `significant_drop_tightening`).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::{parse_totp_input, reveal_totp as reveal_totp_core};

use crate::dto::entry::entry_id_from_str;
use crate::dto::totp::{TotpCodeDto, TotpEnrolmentDto, totp_code_to_dto, totp_enrolment_to_dto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

fn host_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Reveal a Login entry's current TOTP code.
///
/// Audited once per (session, entry); period-boundary refreshes re-generate
/// silently. Only the ephemeral code crosses to WASM — never the seed.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn reveal_totp(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<TotpCodeDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let code = reveal_totp_core(&mut guard, &entry_id_from_str(&entry_id), host_now()).await?;
    Ok(totp_code_to_dto(&code))
}

/// Parse an `otpauth://` URI or a bare Base32 secret into enrolment fields.
///
/// Pure over user input, no session/state: the raw string already came from
/// WASM (the user pasted it), so echoing the normalized secret back is not a
/// core→WASM disclosure. HOTP / `otpauth-migration://` yield distinct errors.
#[tauri::command(rename_all = "snake_case")]
pub async fn parse_totp_enrolment(raw: String) -> Result<TotpEnrolmentDto, CommandError> {
    let enrolment = parse_totp_input(&raw)?;
    Ok(totp_enrolment_to_dto(&enrolment))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::parse_totp_enrolment;
    use crate::error::CommandError;

    #[tokio::test]
    async fn parse_totp_enrolment_ok() {
        let dto = parse_totp_enrolment(
            "otpauth://totp/GitHub:alice?secret=JBSWY3DPEHPK3PXP&digits=8".to_owned(),
        )
        .await
        .unwrap();
        assert_eq!(dto.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(dto.digits, 8);
        assert_eq!(dto.issuer.as_deref(), Some("GitHub"));
    }

    #[tokio::test]
    async fn parse_totp_enrolment_rejects_hotp() {
        let err = parse_totp_enrolment("otpauth://hotp/x?secret=JBSWY3DPEHPK3PXP".to_owned())
            .await
            .unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }

    #[tokio::test]
    async fn parse_totp_enrolment_rejects_migration() {
        let err = parse_totp_enrolment("otpauth-migration://offline?data=abc".to_owned())
            .await
            .unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }
}
