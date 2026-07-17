//! Event helpers.

use wasm_bindgen::JsValue;

/// Read `KeyboardEvent.key` **without crashing** when the property is absent.
///
/// `web_sys::KeyboardEvent::key()` funnels the JS getter through
/// `passStringToWasm0`, which throws `Cannot read properties of undefined
/// (reading 'length')` when `.key` is `undefined` — as it can be on the
/// synthetic keydown events some password managers / assistive tech dispatch on
/// password fields (and which bubble to a Dialog scrim or a document-level
/// Popover listener). Reading reflectively and treating a non-string as empty
/// keeps one keyless event from taking down the whole handler.
pub fn key_of(ev: &web_sys::KeyboardEvent) -> String {
    js_sys::Reflect::get(ev.as_ref(), &JsValue::from_str("key"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}
