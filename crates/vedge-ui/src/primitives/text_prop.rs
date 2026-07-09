use leptos::prelude::*;

/// A reactive string prop that accepts both static strings and reactive signals.
///
/// Use this for any user-facing text that may need i18n support (aria-labels, etc.).
///
/// ```rust
/// // Static (playground, tests)
/// aria_label="Close"
///
/// // Reactive (i18n — updates when locale changes)
/// aria_label=Signal::derive(move || t_string!(i18n, key).to_string())
/// ```
#[derive(Clone, Copy)]
pub struct TextProp(Signal<String>);

impl Default for TextProp {
    fn default() -> Self {
        Self(Signal::stored(String::new()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        Self(Signal::stored(s.to_owned()))
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        Self(Signal::stored(s))
    }
}

impl From<Signal<String>> for TextProp {
    fn from(s: Signal<String>) -> Self {
        Self(s)
    }
}

impl TextProp {
    pub fn get(&self) -> String {
        self.0.get()
    }

    pub fn get_untracked(&self) -> String {
        self.0.get_untracked()
    }
}
