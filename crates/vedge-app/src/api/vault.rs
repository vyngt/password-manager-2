//! Vault lifecycle + read-side queries. Mirrors
//! `vedge-tauri/src/commands/vault.rs`.

use serde::Serialize;

use vedge_ipc::{
    CreateVaultInputDto, CreateVaultOutputDto, IndexEntryDto, TagMetaDto, UnlockVaultInputDto,
};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

pub async fn unlock(input: &UnlockVaultInputDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        input: &'a UnlockVaultInputDto,
    }
    call_void("unlock_vault", &Args { input }).await
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

pub async fn search(vault_path: &str, query: &str) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        query: &'a str,
    }
    call("search", &Args { vault_path, query }).await
}

pub async fn by_tag(vault_path: &str, tag_id: &str) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        tag_id: &'a str,
    }
    call("by_tag", &Args { vault_path, tag_id }).await
}

pub async fn by_folder(
    vault_path: &str,
    folder_id: Option<&str>,
) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder_id: Option<&'a str>,
    }
    call(
        "by_folder",
        &Args {
            vault_path,
            folder_id,
        },
    )
    .await
}

pub async fn by_domain(vault_path: &str, domain: &str) -> Result<Vec<IndexEntryDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        domain: &'a str,
    }
    call("by_domain", &Args { vault_path, domain }).await
}

pub async fn list_tags(vault_path: &str) -> Result<Vec<TagMetaDto>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("list_tags", &Args { vault_path }).await
}
