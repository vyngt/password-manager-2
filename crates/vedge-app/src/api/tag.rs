//! Tag operations. Mirrors `vedge-tauri/src/commands/tag.rs`.

use serde::Serialize;

use vedge_ipc::{CreateTagDto, RenameTagDto};

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

pub async fn create_tag(vault_path: &str, input: &CreateTagDto) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a CreateTagDto,
    }
    call("create_tag", &Args { vault_path, input }).await
}

pub async fn rename_tag(vault_path: &str, input: &RenameTagDto) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a RenameTagDto,
    }
    call_void("rename_tag", &Args { vault_path, input }).await
}

pub async fn delete_tag(vault_path: &str, tag_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        tag_id: &'a str,
    }
    call_void(
        "delete_tag",
        &Args {
            vault_path,
            tag_id,
        },
    )
    .await
}
