use crate::primitives::text_prop::TextProp;

/// Reactive read of a `TextProp` with a static English fallback.
///
/// Call inside a reactive closure (view macro, `Signal::derive`, etc.) —
/// subscribes to the prop so locale switches flow through. The fallback is
/// used only when the prop resolves to an empty string, which is the
/// default for `TextProp::default()`. Consumers who want i18n pass a
/// non-empty `Signal<String>` and never see the fallback.
pub fn text_or(prop: TextProp, fallback: &str) -> String {
    let v = prop.get();
    if v.is_empty() {
        fallback.to_string()
    } else {
        v
    }
}
