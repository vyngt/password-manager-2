//! Entry CRUD + `copy_field` + `move_entry`. Mirrors
//! `vedge-tauri/src/commands/entry.rs`.

use serde::Serialize;

use vedge_ipc::{FieldSelectorDto, PayloadDto};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

pub async fn create_entry(vault_path: &str, payload: &PayloadDto) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        payload: &'a PayloadDto,
    }
    call(
        "create_entry",
        &Args {
            vault_path,
            payload,
        },
    )
    .await
}

/// Reveal one entry's full decrypted payload (secrets included). Backs the
/// edit form's prefill; the backend audits this as `Viewed`.
pub async fn get_entry(vault_path: &str, entry_id: &str) -> Result<PayloadDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call(
        "get_entry",
        &Args {
            vault_path,
            entry_id,
        },
    )
    .await
}

pub async fn update_entry(
    vault_path: &str,
    entry_id: &str,
    payload: &PayloadDto,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        payload: &'a PayloadDto,
    }
    call_void(
        "update_entry",
        &Args {
            vault_path,
            entry_id,
            payload,
        },
    )
    .await
}

pub async fn soft_delete_entry(vault_path: &str, entry_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call_void(
        "soft_delete_entry",
        &Args {
            vault_path,
            entry_id,
        },
    )
    .await
}

pub async fn restore_entry(vault_path: &str, entry_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call_void(
        "restore_entry",
        &Args {
            vault_path,
            entry_id,
        },
    )
    .await
}

pub async fn hard_delete_entry(vault_path: &str, entry_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call_void(
        "hard_delete_entry",
        &Args {
            vault_path,
            entry_id,
        },
    )
    .await
}

pub async fn copy_field(
    vault_path: &str,
    entry_id: &str,
    field: &FieldSelectorDto,
    clear_after_secs: Option<u32>,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        field: &'a FieldSelectorDto,
        #[serde(skip_serializing_if = "Option::is_none")]
        clear_after_secs: Option<u32>,
    }
    call_void(
        "copy_field",
        &Args {
            vault_path,
            entry_id,
            field,
            clear_after_secs,
        },
    )
    .await
}

pub async fn move_entry(
    vault_path: &str,
    entry_id: &str,
    folder_id: Option<&str>,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder_id: Option<&'a str>,
    }
    call_void(
        "move_entry",
        &Args {
            vault_path,
            entry_id,
            folder_id,
        },
    )
    .await
}

/// Flip an entry's `is_favorite` flag. Backend-only mutation (decrypt → set →
/// re-encrypt); no secret fields are returned.
pub async fn set_favorite(
    vault_path: &str,
    entry_id: &str,
    favorite: bool,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        favorite: bool,
    }
    call_void(
        "set_favorite",
        &Args {
            vault_path,
            entry_id,
            favorite,
        },
    )
    .await
}

/// Set an entry's manual ordering position within its folder. Backend-only
/// mutation (decrypt → set → re-encrypt); no secret fields are returned.
pub async fn set_sort_order(vault_path: &str, entry_id: &str, order: u32) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        order: u32,
    }
    call_void(
        "set_sort_order",
        &Args {
            vault_path,
            entry_id,
            order,
        },
    )
    .await
}

/// Replace an entry's tag assignments. Backend-only mutation (decrypt → set →
/// re-encrypt); works for any type, returns no secrets.
pub async fn set_tags(
    vault_path: &str,
    entry_id: &str,
    tag_ids: &[String],
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        tag_ids: &'a [String],
    }
    call_void(
        "set_tags",
        &Args {
            vault_path,
            entry_id,
            tag_ids,
        },
    )
    .await
}
