#[cfg(debug_assertions)]
pub fn assert_supported_type(input_type: &str) {
    // Kept inside its only (debug-gated) consumer so it can never go dead on its
    // own: as a module-level `const` it was unused in release builds (the fn is
    // compiled out), tripping `-D dead-code` — a release-only break that no debug
    // CI job compiled. Caught by PG.2a's `mise release-check`.
    const TIER3_TYPES: &[&str] = &[
        "checkbox",
        "radio",
        "range",
        "file",
        "color",
        "number",
        "date",
        "time",
        "datetime-local",
        "month",
        "week",
    ];
    if TIER3_TYPES.contains(&input_type) {
        web_sys::console::error_1(
            &format!(
                "Input does not support type=\"{input_type}\". Use the dedicated component instead."
            )
            .into(),
        );
    }
}
