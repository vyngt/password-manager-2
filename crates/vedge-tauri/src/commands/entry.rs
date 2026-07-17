//! Entry CRUD + `copy_field` + `move_entry`.
//!
//! See `commands/vault.rs` for the rationale behind the `#![allow]` below
//! (macro expansion of `#[tauri::command(rename_all = "snake_case")]` trips `unreachable` /
//! `let_underscore_must_use`; holding the session mutex across the full
//! command body trips `significant_drop_tightening`).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]
//!
//! Every command here acquires the session's async mutex, converts the
//! DTO input into the domain payload, and delegates to the matching
//! `vedge_core` use case. Secrets cross this boundary as plain `String`s
//! inside the DTO, get wrapped in `SecretString` during conversion, and
//! drop with the DTO as soon as the use case returns.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::instrument;

use vedge_core::domain::shared::{EntryId, TagId, VaultId};
use vedge_core::{
    CopyEnvVarsInput, CopyFieldInput, CopyHistoryFieldInput, CreateEntryInput, GetEntryInput,
    RevealEnvVarsInput, RevealFieldInput, RevealHistoryFieldInput, RevealRecoveryCodesInput,
    UpdateEntryInput, copy_env_vars as copy_env_vars_core, copy_field as copy_field_core,
    copy_history_field as copy_history_field_core, create_entry as create_entry_core,
    get_entry as get_entry_core, get_history_value as get_history_value_core,
    hard_delete_entry as hard_delete_entry_core, list_history as list_history_core,
    move_entry as move_entry_core, resolve_secrets, restore_entry as restore_entry_core,
    restore_from_history as restore_from_history_core, reveal_env_vars as reveal_env_vars_core,
    reveal_field as reveal_field_core, reveal_history_field as reveal_history_field_core,
    reveal_recovery_codes as reveal_recovery_codes_core, set_favorite as set_favorite_core,
    set_sort_order as set_sort_order_core, set_tags as set_tags_core,
    soft_delete_entry as soft_delete_entry_core, update_entry as update_entry_core,
};

use crate::dto::entry::{
    HistoryEntryDto, PayloadDto, entry_id_from_str, history_to_dto, payload_from_dto,
    payload_to_dto, tag_id_from_str,
};
use crate::dto::misc::{
    EnvExportFormatDto, FieldSelectorDto, env_export_format_from_dto, field_selector_from_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Host unix-seconds at the command boundary — injected into the shared TOTP
/// engine so `copy_field` and `reveal_totp` derive the same code.
fn host_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn create_entry(
    vault_path: String,
    payload: PayloadDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    // Create has no prior entry, so sealed-secret intents resolve with `old =
    // None` (required fields must be `Set`) into a complete payload right here —
    // core then takes a fully-materialized payload (slice 5.4).
    let (mut domain_payload, secrets) = payload_from_dto(payload)?;
    resolve_secrets(&mut domain_payload, secrets, None)?;
    let out = create_entry_core(
        &mut guard,
        CreateEntryInput {
            payload: domain_payload,
        },
    )
    .await?;
    Ok(out.entry_id.into_string())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn update_entry(
    vault_path: String,
    entry_id: String,
    payload: PayloadDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    // Split the DTO into a placeholder payload + the sealed-secret intents; the
    // core resolves them against the decrypted old entry (`Unchanged` carries the
    // stored secret forward — slice 5.4).
    let (domain_payload, secrets) = payload_from_dto(payload)?;
    update_entry_core(
        &mut guard,
        UpdateEntryInput {
            entry_id: id,
            payload: domain_payload,
            secrets,
        },
    )
    .await?;
    Ok(())
}

/// Reveal one entry's full decrypted payload (secrets included) to the frontend.
///
/// The sanctioned, user-initiated relaxation of "no plaintext in WASM": used by
/// the edit form to prefill and preserve existing values. The core audits this
/// as `Viewed`.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn get_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PayloadDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let payload = get_entry_core(
        &mut guard,
        GetEntryInput {
            entry_id: entry_id_from_str(&entry_id),
        },
    )
    .await?;
    payload_to_dto(&payload)
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn soft_delete_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    soft_delete_entry_core(&mut guard, &id).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn restore_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    restore_entry_core(&mut guard, &id).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn hard_delete_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    hard_delete_entry_core(&mut guard, &id).await?;
    Ok(())
}

/// `clear_after_secs = None` means "use the use-case default" (30 s). The
/// use case takes a concrete `u32`, so we substitute 30 here.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn copy_field(
    vault_path: String,
    entry_id: String,
    field: FieldSelectorDto,
    clear_after_secs: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    copy_field_core(
        &mut guard,
        CopyFieldInput {
            entry_id: entry_id_from_str(&entry_id),
            field: field_selector_from_dto(field),
            clear_after_secs: clear_after_secs.unwrap_or(30),
        },
        host_now(),
    )
    .await?;
    Ok(())
}

/// Reveal one secret field's plaintext to the renderer (slice 5.4).
///
/// The sanctioned, audited, on-demand relaxation of "no plaintext in WASM" — the
/// twin of `copy_field`, but the value is RETURNED (it crosses to WASM) instead
/// of going to the clipboard. Audits `SecretRevealed`, no dedup.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn reveal_field(
    vault_path: String,
    entry_id: String,
    field: FieldSelectorDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let value = reveal_field_core(
        &mut guard,
        RevealFieldInput {
            entry_id: entry_id_from_str(&entry_id),
            field: field_selector_from_dto(field),
        },
        host_now(),
    )
    .await?;
    Ok((*value).clone())
}

/// Reveal a Login's whole recovery-code list (slice 5.4.1 ②).
///
/// The list crosses to WASM together and is audited as a **single**
/// `SecretRevealed` row — you read recovery codes to print them, not one at a
/// time. Per-code copy uses `copy_field` with `FieldSelector::RecoveryCode(i)`.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn reveal_recovery_codes(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let codes = reveal_recovery_codes_core(
        &mut guard,
        RevealRecoveryCodesInput {
            entry_id: entry_id_from_str(&entry_id),
        },
    )
    .await?;
    Ok(codes.into_iter().map(|c| (*c).clone()).collect())
}

/// Copy an `EnvVars` entry's whole set to the clipboard, `.env` or JSON.
///
/// Slice 5.4.1 ⑥. Formatted SERVER-SIDE and placed on the OS clipboard directly —
/// the plaintext never crosses to WASM. One `SecretRevealed` audit row.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn copy_env_vars(
    vault_path: String,
    entry_id: String,
    format: EnvExportFormatDto,
    clear_after_secs: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    copy_env_vars_core(
        &mut guard,
        CopyEnvVarsInput {
            entry_id: entry_id_from_str(&entry_id),
            format: env_export_format_from_dto(format),
            clear_after_secs: clear_after_secs.unwrap_or(30),
        },
    )
    .await?;
    Ok(())
}

/// Reveal an `EnvVars` entry's whole set to the renderer, `.env` or JSON.
///
/// Slice 5.4.1 ⑥. A `.env` value containing a newline errors
/// (`EnvValueNotDotEnvSafe`) so the UI can point at JSON. One `SecretRevealed`
/// audit row.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn reveal_env_vars(
    vault_path: String,
    entry_id: String,
    format: EnvExportFormatDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let text = reveal_env_vars_core(
        &mut guard,
        RevealEnvVarsInput {
            entry_id: entry_id_from_str(&entry_id),
            format: env_export_format_from_dto(format),
        },
    )
    .await?;
    Ok((*text).clone())
}

/// `folder_id = None` moves the entry to the root. Matches the use case's
/// `Option<&EntryId>` signature.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn move_entry(
    vault_path: String,
    entry_id: String,
    folder_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let folder: Option<EntryId> = folder_id.as_deref().map(entry_id_from_str);
    move_entry_core(&mut guard, &id, folder.as_ref()).await?;
    Ok(())
}

/// Flip an entry's `is_favorite` flag.
///
/// Takes an explicit value (idempotent; the UI knows the current state) and
/// returns nothing — no secret fields cross the boundary, unlike a `get_entry`
/// + `update_entry` round-trip would.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn set_favorite(
    vault_path: String,
    entry_id: String,
    favorite: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    set_favorite_core(&mut guard, &id, favorite).await?;
    Ok(())
}

/// Set an entry's manual ordering position within its folder.
///
/// `sort_order` lives in the encrypted `CommonMeta`, so the backend decrypts →
/// sets → re-encrypts; no secrets cross the boundary (unlike a `get_entry` +
/// `update_entry` round-trip, which would reveal a secret-bearing entry).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn set_sort_order(
    vault_path: String,
    entry_id: String,
    order: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    set_sort_order_core(&mut guard, &id, order).await?;
    Ok(())
}

/// Replace an entry's tag assignments.
///
/// Works for any entry type (Document included) — the tag list lives in the
/// encrypted `CommonMeta`, so the backend decrypts → sets → re-encrypts; no
/// secrets or blob bytes cross the boundary. Dangling ids are tolerated.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn set_tags(
    vault_path: String,
    entry_id: String,
    tag_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let tags: Vec<TagId> = tag_ids.iter().map(|s| tag_id_from_str(s)).collect();
    set_tags_core(&mut guard, &id, tags).await?;
    Ok(())
}

// ---- entry history (slice 2.7) ----------------------------------------------

/// List an entry's version timeline (metadata only — current version + dated
/// snapshots with changed-field summaries). No secret values cross the boundary.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn list_history(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<HistoryEntryDto>, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let versions = list_history_core(&guard, &id).await?;
    Ok(versions.iter().map(history_to_dto).collect())
}

/// Reveal a prior version's full decrypted payload — the sanctioned, audited
/// (`Viewed`) reveal path for history, mirroring `get_entry`.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id, history_id = %history_id))]
pub async fn get_history_value(
    vault_path: String,
    entry_id: String,
    history_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PayloadDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let payload = get_history_value_core(&guard, &id, &history_id).await?;
    payload_to_dto(&payload)
}

/// Copy one field of a prior version to the OS clipboard (auto-clear). Routes
/// through the backend clipboard — the plaintext never returns to WASM.
/// `clear_after_secs = None` uses the 30 s default.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id, history_id = %history_id))]
pub async fn copy_history_field(
    vault_path: String,
    entry_id: String,
    history_id: String,
    field: FieldSelectorDto,
    clear_after_secs: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    copy_history_field_core(
        &guard,
        CopyHistoryFieldInput {
            entry_id: entry_id_from_str(&entry_id),
            history_id,
            field: field_selector_from_dto(field),
            clear_after_secs: clear_after_secs.unwrap_or(30),
        },
        host_now(),
    )
    .await?;
    Ok(())
}

/// Reveal one field of a prior version to the renderer (slice 5.4) — the history
/// twin of `reveal_field`. Returns the plaintext (it crosses to WASM on an
/// explicit click); audits `SecretRevealed`.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id, history_id = %history_id))]
pub async fn reveal_history_field(
    vault_path: String,
    entry_id: String,
    history_id: String,
    field: FieldSelectorDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let value = reveal_history_field_core(
        &guard,
        RevealHistoryFieldInput {
            entry_id: entry_id_from_str(&entry_id),
            history_id,
            field: field_selector_from_dto(field),
        },
        host_now(),
    )
    .await?;
    Ok((*value).clone())
}

/// Restore an entry to a prior version. Server-side re-encrypt (snapshots the
/// now-current value first); no plaintext crosses the boundary.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id, history_id = %history_id))]
pub async fn restore_history(
    vault_path: String,
    entry_id: String,
    history_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    restore_from_history_core(&mut guard, &id, &history_id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )]

    use secrecy::SecretString;
    use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
    use vedge_core::{CreateEntryInput, create_entry, set_favorite};

    use crate::test_support::unlocked_vault;

    #[tokio::test]
    async fn set_favorite_flips_index_flag_through_a_live_session() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "fav.vdb").await;
        let handle = state.get_session(&vault_id).unwrap();
        let mut guard = handle.lock().await;

        let id = create_entry(
            &mut guard,
            CreateEntryInput {
                payload: EntryPayload::Login(LoginPayload {
                    meta: CommonMeta::new("gh", EntryType::Login),
                    username: "alice".into(),
                    password: SecretString::from("hunter2"),
                    totp_secret: None,
                    totp_params: vedge_core::TotpParams::default(),
                    recovery_codes: vec![],
                }),
            },
        )
        .await
        .unwrap()
        .entry_id;

        assert!(!guard.index().entries.get(&id).unwrap().is_favorite);
        set_favorite(&mut guard, &id, true).await.unwrap();
        assert!(guard.index().entries.get(&id).unwrap().is_favorite);
    }
}
