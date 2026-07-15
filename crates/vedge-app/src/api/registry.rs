//! Recent-vaults app.db commands. Mirrors
//! `vedge-tauri/src/commands/recent.rs`.

use serde::Serialize;

use vedge_ipc::{RegisteredVaultDto, RegisteredVaultStatusDto};

use crate::api::call::{call_noargs, call_void};
use crate::api::error::ApiError;

pub async fn list_registered_vaults_with_status() -> Result<Vec<RegisteredVaultStatusDto>, ApiError>
{
    call_noargs("list_registered_vaults_with_status").await
}

pub async fn register_vault(dto: &RegisteredVaultDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        dto: &'a RegisteredVaultDto,
    }
    call_void("register_vault", &Args { dto }).await
}

pub async fn deregister_vault(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("deregister_vault", &Args { id }).await
}

/// Rename a recent vault's display name (inline edit in the vault picker).
pub async fn rename_registered_vault(id: &str, display_name: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
        display_name: &'a str,
    }
    call_void("rename_registered_vault", &Args { id, display_name }).await
}

pub async fn touch_registered_vault_on_unlock(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("touch_registered_vault_on_unlock", &Args { id }).await
}
