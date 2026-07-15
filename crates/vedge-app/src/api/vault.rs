//! Vault lifecycle + read-side queries. Mirrors
//! `vedge-tauri/src/commands/vault.rs`.

use serde::Serialize;

use vedge_ipc::{
    ConvertVaultResultDto, CreateVaultInputDto, CreateVaultOutputDto, IndexEntryDto, TagMetaDto,
    UnlockResultDto, UnlockVaultInputDto,
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

/// Convert a legacy `.vdb` vault to a `<name>.vedge/` home (slice 5.2.0). Returns the new home
/// path (to re-point the registry row) and whether a legacy biometric credential was purged
/// (slice 5.2.3 — the caller then prompts to re-enable Hello in Settings).
pub async fn convert(vault_path: &str) -> Result<ConvertVaultResultDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("convert_vault", &Args { vault_path }).await
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
