//! Vault lifecycle + read-side queries. Mirrors
//! `vedge-tauri/src/commands/vault.rs`.

use serde::Serialize;

use vedge_ipc::{
    CreateVaultInputDto, CreateVaultOutputDto, DeleteVaultReportDto, IndexEntryDto, TagMetaDto,
    UnlockResultDto, UnlockVaultInputDto, VaultDetailsDto,
};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

pub async fn unlock(input: &UnlockVaultInputDto) -> Result<UnlockResultDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UnlockVaultInputDto,
    }
    call("unlock_vault", &Args { input }).await
}

/// Create a new vault. Returns the Emergency-Kit display string + keychain
/// status; the vault ends unlocked in shell state.
pub async fn create(input: &CreateVaultInputDto) -> Result<CreateVaultOutputDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a CreateVaultInputDto,
    }
    call("create_vault", &Args { input }).await
}

pub async fn lock(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call_void("lock_vault", &Args { vault_path }).await
}

/// Destroy a vault — files, all three OS credentials, and its registry row (slice 5.2.4). Refuses
/// an unlocked target. `registry_id` is the recents row id (removed last, as the tombstone). A
/// report with `credentials_cleaned == false` means the UI must warn and name `mise keychain-audit`.
pub async fn delete(
    vault_path: &str,
    registry_id: Option<&str>,
) -> Result<DeleteVaultReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        registry_id: Option<&'a str>,
    }
    call(
        "delete_vault",
        &Args {
            vault_path,
            registry_id,
        },
    )
    .await
}

/// Read-only stats for the vault-details dialog (entries, snapshots, size, uuid). Best-effort.
pub async fn details(vault_path: &str) -> Result<VaultDetailsDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("vault_details", &Args { vault_path }).await
}

/// Whether the backend still holds an unlocked session for this vault.
///
/// `AutoLock` polls this so it notices a **backend-initiated** lock — the hard
/// session TTL (slice 4.5a), or OS screen-lock (4.5b) — and runs the same
/// teardown, wiping decrypted material from WASM. TTL-aware on the backend: an
/// expired-but-not-yet-reaped session already reports `false`.
pub async fn is_unlocked(vault_path: &str) -> Result<bool, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("is_unlocked", &Args { vault_path }).await
}

pub async fn list_entries(vault_path: &str) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("list_entries", &Args { vault_path }).await
}

pub async fn list_trashed(vault_path: &str) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("list_trashed", &Args { vault_path }).await
}

pub async fn list_tags(vault_path: &str) -> Result<Vec<TagMetaDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("list_tags", &Args { vault_path }).await
}
