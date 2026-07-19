//! Re-key (slice 5.8). Mirrors `vedge-tauri/src/commands/rekey.rs`.

use serde::Serialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use vedge_ipc::{RekeyInputDto, RekeyProgressDto, RekeyResultDto};

use crate::api::call::call_void;
use crate::api::error::ApiError;
use crate::api::tauri::{Channel, invoke};

/// Re-key the vault, STREAMING progress over a Tauri `Channel`: `on_progress(done, total)` fires for
/// each entry as the backend re-encrypts (a push, not a poll). On success the vault LOCKS
/// (`RekeyResultDto::cancelled == false`) and the caller navigates to the launch screen;
/// `secret_key_display` is `Some` only when the Secret Key was rotated.
pub async fn rekey_vault(
    vault_path: &str,
    input: &RekeyInputDto,
    mut on_progress: impl FnMut(u64, u64) + 'static,
) -> Result<RekeyResultDto, ApiError> {
    // The progress channel + its message handler. Each backend `send` calls the handler with the
    // deserialized `RekeyProgressDto`. The closure is kept alive until `invoke` resolves — i.e. for
    // the whole re-key — so every tick is delivered.
    let channel = Channel::new();
    let handler = Closure::wrap(Box::new(move |msg: JsValue| {
        if let Ok(p) = serde_wasm_bindgen::from_value::<RekeyProgressDto>(msg) {
            on_progress(p.done, p.total);
        }
    }) as Box<dyn FnMut(JsValue)>);
    channel.set_onmessage(&handler);

    // Build the args object so the raw Channel can ride alongside the serde fields; Tauri matches it
    // to the command's `on_progress` param by name.
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a RekeyInputDto,
    }
    let args =
        serde_wasm_bindgen::to_value(&Args { vault_path, input }).map_err(ApiError::serialize)?;
    js_sys::Reflect::set(
        &args,
        &JsValue::from_str("on_progress"),
        &JsValue::from(channel),
    )
    .map_err(|e| ApiError::from_rejection(&e))?;

    let raw = invoke("rekey_vault", args).await;
    // Keep the onmessage closure alive across the whole command, then drop it.
    drop(handler);
    let raw = raw.map_err(|e| ApiError::from_rejection(&e))?;
    serde_wasm_bindgen::from_value::<RekeyResultDto>(raw).map_err(ApiError::deserialize)
}

/// Signal an in-flight re-key to cancel (before its commit point).
pub async fn cancel_rekey(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call_void("cancel_rekey", &Args { vault_path }).await
}
