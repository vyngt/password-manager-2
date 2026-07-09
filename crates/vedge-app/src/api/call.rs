//! Generic IPC dispatch helpers. Every typed `api::*` function in this
//! crate goes through exactly one of these three entry points; no
//! component calls `invoke()` directly.

use serde::{Serialize, de::DeserializeOwned};

use crate::api::error::ApiError;
use crate::api::tauri::invoke;

/// Serialize `args`, dispatch the command, deserialize the response.
///
/// Three failure classes:
/// - `Serialize` → bug in the caller (DTO drift / bad input).
/// - `Deserialize` → shell returned something the frontend didn't expect.
/// - Promise rejection → decoded via [`ApiError::from_rejection`] into a
///   shell-reported variant (`WrongCredentials`, `NotFound`, …).
pub async fn call<I, O>(cmd: &str, args: &I) -> Result<O, ApiError>
where
    I: Serialize + ?Sized,
    O: DeserializeOwned,
{
    let args_js = serde_wasm_bindgen::to_value(args).map_err(ApiError::serialize)?;
    let raw = invoke(cmd, args_js)
        .await
        .map_err(|e| ApiError::from_rejection(&e))?;
    serde_wasm_bindgen::from_value::<O>(raw).map_err(ApiError::deserialize)
}

/// For commands that return `()`. Tauri serializes unit as `null`;
/// `serde_wasm_bindgen::from_value::<()>` accepts it.
pub async fn call_void<I>(cmd: &str, args: &I) -> Result<(), ApiError>
where
    I: Serialize + ?Sized,
{
    call::<I, ()>(cmd, args).await
}

/// For commands taking no arguments. Tauri expects an empty object (not
/// `null`); the unit struct `NoArgs` serializes to `{}`.
pub async fn call_noargs<O>(cmd: &str) -> Result<O, ApiError>
where
    O: DeserializeOwned,
{
    #[derive(Serialize)]
    struct NoArgs {}
    call(cmd, &NoArgs {}).await
}
