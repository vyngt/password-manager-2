//! App-global security preferences: idle auto-lock, lock-on-blur, and the
//! clipboard clear delay.
//!
//! Persisted in the generic `app_settings` KV store under a single fixed key
//! (`security.prefs`) — app-global, not per-vault (unlike 2.5.3's smart folders,
//! which key by vault path). Provided as a reactive [`SecurityPrefsCtx`] at the
//! app root so the [`AutoLock`](crate::features::vault::auto_lock::AutoLock)
//! hook and both clipboard copy sites read it live; the Settings **Security**
//! section edits and saves it.

use crate::api;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

/// `app_settings` key holding the serialized [`SecurityPrefs`].
pub const KEY: &str = "security.prefs";

/// The three idle-security preferences. Every field carries a `#[serde(default)]`
/// so a missing key (or an older partial blob) loads with sensible defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPrefs {
    /// Idle minutes before the vault auto-locks; `0` disables auto-lock.
    #[serde(default = "default_auto_lock_minutes")]
    pub auto_lock_minutes: u32,
    /// Lock when the app window loses focus (switching apps).
    #[serde(default)]
    pub lock_on_blur: bool,
    /// Seconds after which a copied field is wiped from the clipboard.
    #[serde(default = "default_clipboard_clear_seconds")]
    pub clipboard_clear_seconds: u32,
}

fn default_auto_lock_minutes() -> u32 {
    15
}
fn default_clipboard_clear_seconds() -> u32 {
    30
}

impl Default for SecurityPrefs {
    fn default() -> Self {
        Self {
            auto_lock_minutes: default_auto_lock_minutes(),
            lock_on_blur: false,
            clipboard_clear_seconds: default_clipboard_clear_seconds(),
        }
    }
}

/// Reactive context wrapper — the single live copy of the prefs, provided at the
/// app root and read via `expect_context::<SecurityPrefsCtx>()`.
#[derive(Clone, Copy)]
pub struct SecurityPrefsCtx(pub RwSignal<SecurityPrefs>);

/// Flipped to `true` once [`load`] resolves. The Settings **Security** section
/// gates its controls behind this (with a `Spinner` fallback) so they don't
/// flash the [`SecurityPrefs::default`] seed before the persisted values arrive.
#[derive(Clone, Copy)]
pub struct SecurityPrefsLoaded(pub RwSignal<bool>);

/// Load the persisted prefs, falling back to [`SecurityPrefs::default`] when the
/// key is absent or unreadable.
pub async fn load() -> SecurityPrefs {
    match api::settings::get_app_setting(KEY).await {
        Ok(Some(dto)) => serde_json::from_value(dto.value).unwrap_or_default(),
        _ => SecurityPrefs::default(),
    }
}

/// Persist the prefs (fire-and-forget; a write error is non-fatal to the UI —
/// the in-memory signal already reflects the change).
pub fn save(prefs: SecurityPrefs) {
    if let Ok(value) = serde_json::to_value(&prefs) {
        spawn_local(async move {
            let _ = api::settings::set_app_setting(KEY, &value).await;
        });
    }
}

/// Idle window in milliseconds for `minutes` of inactivity; `0` → `None` (off).
/// Pure so it stays host-testable.
#[must_use]
pub fn idle_ms(minutes: u32) -> Option<u64> {
    (minutes > 0).then(|| u64::from(minutes) * 60_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn idle_ms_off_and_on() {
        assert_eq!(idle_ms(0), None);
        assert_eq!(idle_ms(1), Some(60_000));
        assert_eq!(idle_ms(15), Some(900_000));
    }

    #[test]
    fn serde_round_trip() {
        let prefs = SecurityPrefs {
            auto_lock_minutes: 5,
            lock_on_blur: true,
            clipboard_clear_seconds: 45,
        };
        let value = serde_json::to_value(&prefs).unwrap();
        let back: SecurityPrefs = serde_json::from_value(value).unwrap();
        assert_eq!(prefs, back);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let empty: SecurityPrefs = serde_json::from_value(json!({})).unwrap();
        assert_eq!(empty, SecurityPrefs::default());
        assert_eq!(empty.auto_lock_minutes, 15);
        assert_eq!(empty.clipboard_clear_seconds, 30);
        assert!(!empty.lock_on_blur);

        let partial: SecurityPrefs =
            serde_json::from_value(json!({ "auto_lock_minutes": 5 })).unwrap();
        assert_eq!(partial.auto_lock_minutes, 5);
        assert!(!partial.lock_on_blur);
        assert_eq!(partial.clipboard_clear_seconds, 30);
    }
}
