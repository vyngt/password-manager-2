//! Recent-vaults app.db commands. Mirrors
//! `vedge-tauri/src/commands/recent.rs`.

use serde::Serialize;

use vedge_ipc::{RecentVaultDto, RecentVaultStatusDto};

use crate::api::call::{call_noargs, call_void};
use crate::api::error::ApiError;

pub async fn list_recent_vaults_with_status() -> Result<Vec<RecentVaultStatusDto>, ApiError> {
    call_noargs("list_recent_vaults_with_status").await
}

pub async fn add_recent_vault(dto: &RecentVaultDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        dto: &'a RecentVaultDto,
    }
    call_void("add_recent_vault", &Args { dto }).await
}

pub async fn remove_recent_vault(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("remove_recent_vault", &Args { id }).await
}

/// Rename a recent vault's display name (inline edit in the vault picker).
pub async fn rename_recent_vault(id: &str, display_name: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
        display_name: &'a str,
    }
    call_void("rename_recent_vault", &Args { id, display_name }).await
}

pub async fn touch_recent_vault_on_unlock(id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call_void("touch_recent_vault_on_unlock", &Args { id }).await
}
