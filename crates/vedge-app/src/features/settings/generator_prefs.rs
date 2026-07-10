//! App-global password-generator preferences: the selected mode + per-mode presets.
//!
//! Persisted in the generic `app_settings` KV store under a single fixed key
//! (`generator.prefs`) — app-global, not per-vault (a generation preference
//! isn't vault-specific). Provided as a reactive [`GeneratorPrefsCtx`] at the app
//! root so the inline quick-generate button, the expand-to-tune popover, and the
//! standalone `/v/generator` panel all share one preset.
//!
//! `GeneratorPrefs` is the app-side **serde mirror** of the engine's config
//! types, keeping `vedge-generator` serde-free. `#[serde(default)]` on every
//! field means an older/partial blob still loads. It stays `Copy` — the free-text
//! pattern string is a session-local signal in the panel, not persisted here, so
//! this struct holds only small `Copy` fields.

use crate::api;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use vedge_generator::{
    CharClasses, PassphraseConfig, PinConfig, PronounceableConfig, RandomConfig, Separator,
};

/// `app_settings` key holding the serialized [`GeneratorPrefs`].
pub const KEY: &str = "generator.prefs";

/// Which generator mode the panel shows. Serde mirror of the engine's `GenSpec`
/// discriminant; also drives the mode-picker `SegmentedControl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GenMode {
    /// Random charset (also the inline Login wand).
    #[default]
    Random,
    /// EFF-wordlist passphrase.
    Passphrase,
    /// Digits-only PIN.
    Pin,
    /// Consonant/vowel pronounceable.
    Pronounceable,
    /// Token-grammar pattern.
    Pattern,
}

impl GenMode {
    /// All modes, in picker order.
    pub const ALL: [Self; 5] = [
        Self::Random,
        Self::Passphrase,
        Self::Pin,
        Self::Pronounceable,
        Self::Pattern,
    ];

    /// Stable string used as the `SegmentedControl` option value.
    #[must_use]
    pub const fn as_value(self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Passphrase => "passphrase",
            Self::Pin => "pin",
            Self::Pronounceable => "pronounceable",
            Self::Pattern => "pattern",
        }
    }

    /// Parse an option value back to a mode (unknown → `Random`).
    #[must_use]
    pub fn from_value(v: &str) -> Self {
        match v {
            "passphrase" => Self::Passphrase,
            "pin" => Self::Pin,
            "pronounceable" => Self::Pronounceable,
            "pattern" => Self::Pattern,
            _ => Self::Random,
        }
    }
}

/// Serde mirror of the engine's `Separator` (keeps `vedge-generator` serde-free).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SeparatorPref {
    /// `-` (default).
    #[default]
    Hyphen,
    /// ` `
    Space,
    /// `.`
    Dot,
    /// `_`
    Underscore,
    /// no separator
    None,
}

impl SeparatorPref {
    /// All separators, in `Select` order.
    pub const ALL: [Self; 5] = [
        Self::Hyphen,
        Self::Space,
        Self::Dot,
        Self::Underscore,
        Self::None,
    ];

    /// Stable string used as the `Select` option value.
    #[must_use]
    pub const fn as_value(self) -> &'static str {
        match self {
            Self::Hyphen => "hyphen",
            Self::Space => "space",
            Self::Dot => "dot",
            Self::Underscore => "underscore",
            Self::None => "none",
        }
    }

    /// Parse an option value back to a separator (unknown → `Hyphen`).
    #[must_use]
    pub fn from_value(v: &str) -> Self {
        match v {
            "space" => Self::Space,
            "dot" => Self::Dot,
            "underscore" => Self::Underscore,
            "none" => Self::None,
            _ => Self::Hyphen,
        }
    }

    /// Convert to the engine's `Separator`.
    #[must_use]
    pub const fn to_engine(self) -> Separator {
        match self {
            Self::Hyphen => Separator::Hyphen,
            Self::Space => Separator::Space,
            Self::Dot => Separator::Dot,
            Self::Underscore => Separator::Underscore,
            Self::None => Separator::None,
        }
    }
}

/// Flat serde mirror of the generator config. Every field carries a
/// `#[serde(default)]` so a missing key (or an older partial blob) loads with
/// defaults. The random fields alone still satisfy `RandomConfig::from`, so the
/// inline Login wand is unaffected by the 3.4 additions.
// Eight bools is the domain (four random classes + two random options + two
// passphrase options), not a modelling smell.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorPrefs {
    // ----- random mode (the inline Login wand always reads these) -----
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

    // ----- 3.4: selected mode + per-mode presets -----
    #[serde(default)]
    pub mode: GenMode,
    #[serde(default = "default_words")]
    pub words: u32,
    #[serde(default)]
    pub separator: SeparatorPref,
    #[serde(default)]
    pub capitalize: bool,
    #[serde(default)]
    pub include_number: bool,
    #[serde(default = "default_pin_length")]
    pub pin_length: u32,
    #[serde(default = "default_pron_length")]
    pub pron_length: u32,
}

fn default_length() -> u32 {
    20
}
fn default_true() -> bool {
    true
}
fn default_words() -> u32 {
    6
}
fn default_pin_length() -> u32 {
    6
}
fn default_pron_length() -> u32 {
    12
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
            mode: GenMode::Random,
            words: default_words(),
            separator: SeparatorPref::Hyphen,
            capitalize: false,
            include_number: false,
            pin_length: default_pin_length(),
            pron_length: default_pron_length(),
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

impl GeneratorPrefs {
    /// Build the passphrase config from the persisted passphrase fields.
    #[must_use]
    pub fn passphrase_config(self) -> PassphraseConfig {
        PassphraseConfig {
            words: self.words,
            separator: self.separator.to_engine(),
            capitalize: self.capitalize,
            include_number: self.include_number,
        }
    }

    /// Build the PIN config from the persisted PIN length.
    #[must_use]
    pub const fn pin_config(self) -> PinConfig {
        PinConfig {
            length: self.pin_length,
        }
    }

    /// Build the pronounceable config from the persisted length.
    #[must_use]
    pub const fn pron_config(self) -> PronounceableConfig {
        PronounceableConfig {
            length: self.pron_length,
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
        // GeneratorPrefs is now a superset of RandomConfig; the random fields
        // must still round-trip losslessly (the inline Login wand depends on it).
        // Non-random fields start at defaults, which `From<RandomConfig>` restores.
        let p = GeneratorPrefs {
            length: 32,
            uppercase: false,
            symbols: false,
            require_each_selected: true,
            exclude_ambiguous: true,
            ..Default::default()
        };
        assert_eq!(GeneratorPrefs::from(RandomConfig::from(p)), p);
    }

    #[test]
    fn per_mode_config_builders() {
        let p = GeneratorPrefs {
            words: 8,
            separator: SeparatorPref::Dot,
            capitalize: true,
            include_number: true,
            pin_length: 4,
            pron_length: 16,
            ..Default::default()
        };

        assert_eq!(p.passphrase_config().words, 8);
        assert_eq!(p.passphrase_config().separator, Separator::Dot);
        assert!(p.passphrase_config().capitalize);
        assert!(p.passphrase_config().include_number);
        assert_eq!(p.pin_config().length, 4);
        assert_eq!(p.pron_config().length, 16);
    }

    #[test]
    fn mode_and_separator_value_round_trip() {
        for m in GenMode::ALL {
            assert_eq!(GenMode::from_value(m.as_value()), m);
        }
        for s in SeparatorPref::ALL {
            assert_eq!(SeparatorPref::from_value(s.as_value()), s);
        }
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
            mode: GenMode::Passphrase,
            words: 9,
            separator: SeparatorPref::Underscore,
            capitalize: true,
            include_number: true,
            pin_length: 8,
            pron_length: 18,
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
        // 3.4 fields default too.
        assert_eq!(empty.mode, GenMode::Random);
        assert_eq!(empty.words, 6);
        assert_eq!(empty.separator, SeparatorPref::Hyphen);
        assert!(!empty.capitalize && !empty.include_number);
        assert_eq!(empty.pin_length, 6);
        assert_eq!(empty.pron_length, 12);

        // A partial blob keeps its present fields and defaults the rest.
        let partial: GeneratorPrefs =
            serde_json::from_value(json!({ "length": 8, "mode": "Passphrase", "words": 4 }))
                .unwrap();
        assert_eq!(partial.length, 8);
        assert_eq!(partial.mode, GenMode::Passphrase);
        assert_eq!(partial.words, 4);
        assert!(partial.lowercase); // defaulted true
        assert_eq!(partial.pin_length, 6); // defaulted
    }
}
