//! Password-health command (slice 4.3): the vault-wide secret scan.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion of
//! `#[tauri::command]` trips `unreachable` / `let_underscore_must_use`; holding
//! the session mutex trips `significant_drop_tightening`).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use serde::Deserialize;
use tracing::instrument;

use vedge_core::application::app::ports::AppSettingRepository;
use vedge_core::application::vault::ports::BreachChecker;
use vedge_core::domain::shared::VaultId;
use vedge_core::{get_app_setting, scan_health as scan_health_core};

use crate::dto::health::{
    HealthReportDto, HealthScanInputDto, health_report_to_dto, health_scan_input_from_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// The app-global security-prefs blob key. Owned by the frontend
/// (`features/settings/security_prefs.rs`); read here so the **backend** — not the
/// renderer — verifies opt-in before any password-derived data egresses.
const SECURITY_PREFS_KEY: &str = "security.prefs";

/// The one field of `SecurityPrefs` the shell needs. Serde ignores the rest; a
/// missing key or malformed blob resolves to `false` (fail-safe — never egress
/// without explicit, readable consent).
#[derive(Deserialize, Default)]
struct BreachPref {
    #[serde(default)]
    breach_check_enabled: bool,
}

/// Read the persisted opt-in flag. Defaults to disabled on any read/parse failure.
async fn breach_opt_in(app_settings: &dyn AppSettingRepository) -> bool {
    match get_app_setting(app_settings, SECURITY_PREFS_KEY).await {
        Ok(Some(row)) => {
            serde_json::from_value::<BreachPref>(row.value).is_ok_and(|p| p.breach_check_enabled)
        }
        _ => false,
    }
}

/// Run a vault-wide password-health scan.
///
/// Decrypts every entry server-side and returns **only derivatives** (zxcvbn
/// scores, reuse-group ordinals, ages) — no secret crosses to WASM. Audit-silent
/// per entry; emits exactly one `HealthScanned` row. The slowest command in the
/// app (a full-vault decrypt), so the caller shows a spinner.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn scan_health(
    vault_path: String,
    input: HealthScanInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<HealthReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;

    // The BACKEND verifies consent (Decision 3): a password's SHA-1 prefix only
    // egresses if the user has turned the check on. A quick app.db read, done
    // before taking the session lock.
    let breach: Option<&dyn BreachChecker> = if breach_opt_in(&*state.app_settings).await {
        Some(state.breach.as_ref())
    } else {
        None
    };

    let guard = handle.lock().await;
    let report = scan_health_core(&guard, health_scan_input_from_dto(input), breach).await?;
    Ok(health_report_to_dto(&report))
}
