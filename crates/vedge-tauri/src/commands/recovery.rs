//! `unlock_with_recovery_key` — unlock a vault using a typed Emergency
//! Kit string and (on success) re-register the Secret Key in the OS
//! keychain.
//!
//! Returns a small DTO describing whether the keychain write succeeded.
//! The session itself is inserted into `AppState`, matching the plain
//! `unlock_vault` command.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::KeychainProvider;
use vedge_core::domain::shared::VaultId;
use vedge_core::{RecoverVaultInput, recover_vault as core_recover};

use crate::dto::emergency_kit::{
    RecoveryOutcomeDto, UnlockWithRecoveryKeyInputDto, recovery_outcome_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %input.vault_path))]
pub async fn unlock_with_recovery_key(
    input: UnlockWithRecoveryKeyInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<RecoveryOutcomeDto, CommandError> {
    let vault_path = PathBuf::from(&input.vault_path);
    let vault_id = VaultId::new(vault_path.clone());

    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(format!(
            "vault already unlocked: {}",
            vault_path.display()
        )));
    }

    let master_password = Zeroizing::new(input.master_password);
    let outcome = core_recover(
        &state.unlock_vault,
        Arc::clone(&state.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path,
            master_password,
            recovery_key_display: input.recovery_key_display,
        },
    )
    .await?;

    let dto = recovery_outcome_to_dto(&outcome);
    state.insert_session(vault_id, outcome.session)?;
    Ok(dto)
}
