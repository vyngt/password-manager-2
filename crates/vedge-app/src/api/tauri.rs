use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // `catch` translates Tauri's promise rejections (which carry the
    // shell's `CommandError` envelope) into `Err(JsValue)` instead of
    // panicking the WASM module. Every typed wrapper in `api/call.rs`
    // relies on this.
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Tauri v2 `Channel` — the streaming primitive for a long-running command (slice 5.8, re-key
/// progress). Exposed globally by `withGlobalTauri` at `window.__TAURI__.core.Channel`. A command
/// with a `Channel<T>` param pushes messages to the JS `onmessage` handler as it runs; the frontend
/// passes the channel in the invoke args under the param's name.
#[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
extern "C" {
    pub type Channel;

    #[wasm_bindgen(constructor)]
    pub fn new() -> Channel;

    /// Set the message handler (`channel.onmessage = handler`). Each backend `send` invokes it with
    /// the deserialized payload.
    #[wasm_bindgen(method, setter = onmessage)]
    pub fn set_onmessage(
        this: &Channel,
        handler: &::wasm_bindgen::closure::Closure<dyn FnMut(JsValue)>,
    );
}

/// Window APIs
#[wasm_bindgen]
extern "C" {

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "window"], js_name = Window)]
    pub type TauriWindow;

    #[wasm_bindgen(method, js_name = isMaximized)]
    pub async fn is_maximized(this: &TauriWindow) -> JsValue;

    #[wasm_bindgen(method, js_name = maximize)]
    pub async fn maximize(this: &TauriWindow);

    #[wasm_bindgen(method, js_name = unmaximize)]
    pub async fn unmaximize(this: &TauriWindow);

    #[wasm_bindgen(method, js_name = minimize)]
    pub async fn minimize(this: &TauriWindow);

    #[wasm_bindgen(method, js_name = close)]
    pub async fn close(this: &TauriWindow);

    #[wasm_bindgen(method, js_name = startDragging)]
    pub async fn start_dragging(this: &TauriWindow);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "window"], js_name = getCurrentWindow)]
    pub fn get_current_window() -> TauriWindow;
}

/// Dialog plugin APIs. Exposed under `window.__TAURI__.dialog` because
/// `withGlobalTauri` is enabled and `tauri_plugin_dialog` is registered.
#[wasm_bindgen]
extern "C" {
    // `save` shows a native save-file dialog and resolves to the chosen path
    // string, or `null` if the user cancels. `catch` maps a rejection to
    // `Err(JsValue)`.
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "dialog"], js_name = save)]
    pub async fn dialog_save(options: JsValue) -> Result<JsValue, JsValue>;

    // `open` shows a native open-file dialog. With no `multiple`/`directory`
    // option it resolves to a single path string, or `null` if cancelled.
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "dialog"], js_name = open)]
    pub async fn dialog_open(options: JsValue) -> Result<JsValue, JsValue>;
}
