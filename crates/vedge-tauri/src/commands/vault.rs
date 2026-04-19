//! Session-lifecycle + read-side commands.
//!
//! - `unlock_vault` / `lock_vault` / `is_unlocked`
//! - Non-mutating queries via the vault query use cases:
//!   `list_entries`, `list_trashed`, `search`, `by_tag`, `by_folder`,
//!   `by_domain`, `list_tags`.
//!
//! Every command routes through a `vedge-core` use case. The shell never
//! touches `VaultRepository` or `VaultIndex` methods directly — the strict
//! "every command via a use case" rule is enforced here.
//!
//! ## Allow block below
//!
//! The `#[tauri::command]` attribute macro expands into dispatch code that
//! includes an `unreachable!()` branch (argument-count mismatch) and a
//! `let _ = <must_use>` on the handler return. Both trip the workspace's
//! `deny(unreachable, let_underscore_must_use)` under `cargo clippy
//! --all-targets -- -D warnings`. The `significant_drop_tightening` lint
//! (nursery, promoted by `-D warnings`) also fires because every command
//! holds the async mutex guard for its full body — which is correct:
//! the guard must outlive the `&mut session` / `&VaultIndex` passed to
//! the use case.
//!
//! Repro: `cargo clippy -p vedge-tauri --all-targets -- -D warnings`
//! without this allow yields 92+ errors, all macro-generated.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use vedge_core::domain::shared::VaultId;
use vedge_core::{
    entries_by_domain, entries_by_folder, entries_by_tag, list_active_entries,
    list_tags as list_tags_core, list_trashed_entries, lock_vault as lock_vault_core,
    search_entries, UnlockVaultInput,
};

use crate::dto::entry::{entry_id_from_str, tag_id_from_str, IndexEntryDto};
use crate::dto::misc::UnlockVaultInputDto;
use crate::dto::tag::TagMetaDto;
use crate::error::CommandError;
use crate::state::AppState;

// ---- helpers ----------------------------------------------------------------

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

// ---- unlock / lock / is_unlocked --------------------------------------------

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %input.vault_path))]
pub async fn unlock_vault(
    input: UnlockVaultInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_path = PathBuf::from(&input.vault_path);
    let vault_id = VaultId::new(vault_path.clone());

    // Refuse a double-unlock — caller must lock first.
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(format!(
            "vault already unlocked: {}",
            vault_path.display()
        )));
    }

    let secret_key = input.decode_secret_key()?;
    let master_password = Zeroizing::new(input.master_password.clone());

    // The use case constructs the per-vault repo + blob store via its
    // factory ports; the shell never touches infrastructure directly.
    let session = state
        .unlock_vault
        .execute(UnlockVaultInput {
            vault_path,
            master_password,
            secret_key,
        })
        .await?;

    state.insert_session(vault_id, session)?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn lock_vault(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);

    // Remove first so any concurrent `get_session` starts returning
    // `VaultNotOpen`; in-flight clones of the Arc held by other commands
    // will drop as those commands finish.
    let Some(arc) = state.remove_session(&vault_id) else {
        return Err(CommandError::VaultNotOpen(vault_path));
    };

    // Spin-retry to gain exclusive ownership. In practice other commands
    // release their Arc within a few await points.
    const MAX_RETRIES: usize = 256;
    let mut arc = arc;
    for _ in 0..MAX_RETRIES {
        match Arc::try_unwrap(arc) {
            Ok(mutex) => {
                let session = mutex.into_inner();
                return lock_vault_core(session).await.map_err(Into::into);
            }
            Err(still_shared) => {
                arc = still_shared;
                tokio::task::yield_now().await;
            }
        }
    }

    // Last resort: drop our reference and let the remaining clones finish.
    // `VaultSession::Drop` still runs when the last clone drops, so the
    // KEK + index get zeroized. We lose only the `AuditAction::Locked` row.
    drop(arc);
    tracing::warn!("lock_vault: timed out waiting for concurrent commands; audit event skipped");
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn is_unlocked(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, CommandError> {
    Ok(state.is_unlocked(&vault_id_from_string(&vault_path)))
}

// ---- read queries (via use cases) -------------------------------------------

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn list_entries(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(list_active_entries(guard.index())
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn list_trashed(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(list_trashed_entries(guard.index())
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn search(
    vault_path: String,
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(search_entries(guard.index(), &query)
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, tag_id = %tag_id))]
pub async fn by_tag(
    vault_path: String,
    tag_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let tid = tag_id_from_str(&tag_id);
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(entries_by_tag(guard.index(), &tid)
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn by_folder(
    vault_path: String,
    folder_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let folder = folder_id.as_deref().map(entry_id_from_str);
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(entries_by_folder(guard.index(), folder.as_ref())
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, domain = %domain))]
pub async fn by_domain(
    vault_path: String,
    domain: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IndexEntryDto>, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(entries_by_domain(guard.index(), &domain)
        .await
        .iter()
        .map(IndexEntryDto::from)
        .collect())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn list_tags(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TagMetaDto>, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    Ok(list_tags_core(guard.index())
        .await
        .iter()
        .map(TagMetaDto::from)
        .collect())
}
