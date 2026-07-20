//! Change password. Mirrors `vedge-tauri/src/commands/password.rs`.

use serde::Serialize;

use vedge_ipc::ChangePasswordInputDto;

use crate::api::call::call_void;
use crate::api::error::ApiError;

pub async fn change_password(
    vault_path: &str,
    input: &ChangePasswordInputDto,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a ChangePasswordInputDto,
    }
    call_void("change_password", &Args { vault_path, input }).await
}
