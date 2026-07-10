//! App-global password-generator preferences: the last-used charset preset.
//!
//! Persisted in the generic `app_settings` KV store under a single fixed key
//! (`generator.prefs`) — app-global, not per-vault (a generation preference
//! isn't vault-specific). Provided as a reactive [`GeneratorPrefsCtx`] at the app
//! root so the inline quick-generate button, the expand-to-tune popover, and the
//! standalone `/v/generator` panel all share one preset.
//!
//! `GeneratorPrefs` is the app-side **serde mirror** of the engine's
//! [`RandomConfig`], keeping `vedge-generator` serde-free. `#[serde(default)]` on
//! every field means an older/partial blob still loads.

use crate::api;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use vedge_generator::{CharClasses, RandomConfig};

/// `app_settings` key holding the serialized [`GeneratorPrefs`].
pub const KEY: &str = "generator.prefs";

/// Flat serde mirror of [`RandomConfig`]. Every field carries a
/// `#[serde(default)]` so a missing key (or an older partial blob) loads with
/// defaults that match [`RandomConfig::default`].
// Four class bools + two option bools is the domain (mirrors the engine's
// `RandomConfig`/`CharClasses`), not a modelling smell.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorPrefs {
    #[serde(default = "default_length")]
    pub length: u32,
    #[serde(default = "default_true")]
    pub lowercase: bool,
    #[serde(default = "default_true")]
    pub uppercase: bool,
    #[serde(default = "default_true")]
    pub digits: bool,
    #[serde(default = "default_true")]
    pub symbols: bool,
    #[serde(default)]
    pub require_each_selected: bool,
    #[serde(default)]
    pub exclude_ambiguous: bool,
}

fn default_length() -> u32 {
    20
}
fn default_true() -> bool {
    true
}

impl Default for GeneratorPrefs {
    fn default() -> Self {
        Self::from(RandomConfig::default())
    }
}

impl From<RandomConfig> for GeneratorPrefs {
    fn from(c: RandomConfig) -> Self {
        Self {
            length: c.length,
            lowercase: c.classes.lowercase,
            uppercase: c.classes.uppercase,
            digits: c.classes.digits,
            symbols: c.classes.symbols,
            require_each_selected: c.require_each_selected,
            exclude_ambiguous: c.exclude_ambiguous,
        }
    }
}

impl From<GeneratorPrefs> for RandomConfig {
    fn from(p: GeneratorPrefs) -> Self {
        Self {
            length: p.length,
            classes: CharClasses {
                lowercase: p.lowercase,
                uppercase: p.uppercase,
                digits: p.digits,
                symbols: p.symbols,
            },
            require_each_selected: p.require_each_selected,
            exclude_ambiguous: p.exclude_ambiguous,
        }
    }
}

/// Reactive context wrapper — the single live copy of the preset, provided at the
/// app root and read via `expect_context::<GeneratorPrefsCtx>()`.
#[derive(Clone, Copy)]
pub struct GeneratorPrefsCtx(pub RwSignal<GeneratorPrefs>);

/// Load the persisted preset, falling back to [`GeneratorPrefs::default`] when the
/// key is absent or unreadable.
pub async fn load() -> GeneratorPrefs {
    match api::settings::get_app_setting(KEY).await {
        Ok(Some(dto)) => serde_json::from_value(dto.value).unwrap_or_default(),
        _ => GeneratorPrefs::default(),
    }
}

/// Persist the preset (fire-and-forget; a write error is non-fatal to the UI —
/// the in-memory signal already reflects the change).
pub fn save(prefs: &GeneratorPrefs) {
    if let Ok(value) = serde_json::to_value(prefs) {
        spawn_local(async move {
            let _ = api::settings::set_app_setting(KEY, &value).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_maps_to_random_config_default() {
        assert_eq!(
            RandomConfig::from(GeneratorPrefs::default()),
            RandomConfig::default()
        );
        assert_eq!(
            GeneratorPrefs::default(),
            GeneratorPrefs::from(RandomConfig::default())
        );
    }

    #[test]
    fn round_trips_through_random_config() {
        let p = GeneratorPrefs {
            length: 32,
            lowercase: true,
            uppercase: false,
            digits: true,
            symbols: false,
            require_each_selected: true,
            exclude_ambiguous: true,
        };
        assert_eq!(GeneratorPrefs::from(RandomConfig::from(p)), p);
    }

    #[test]
    fn serde_round_trip() {
        let p = GeneratorPrefs {
            length: 24,
            lowercase: false,
            uppercase: true,
            digits: false,
            symbols: true,
            require_each_selected: true,
            exclude_ambiguous: false,
        };
        let value = serde_json::to_value(p).unwrap();
        let back: GeneratorPrefs = serde_json::from_value(value).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let empty: GeneratorPrefs = serde_json::from_value(json!({})).unwrap();
        assert_eq!(empty, GeneratorPrefs::default());
        assert_eq!(empty.length, 20);
        assert!(empty.lowercase && empty.uppercase && empty.digits && empty.symbols);
        assert!(!empty.require_each_selected && !empty.exclude_ambiguous);

        // A partial blob keeps its present fields and defaults the rest.
        let partial: GeneratorPrefs =
            serde_json::from_value(json!({ "length": 8, "symbols": false })).unwrap();
        assert_eq!(partial.length, 8);
        assert!(!partial.symbols);
        assert!(partial.lowercase); // defaulted true
        assert!(!partial.exclude_ambiguous);
    }
}
