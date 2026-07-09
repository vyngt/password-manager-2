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

#[cfg(debug_assertions)]
pub fn assert_supported_type(input_type: &str) {
    if TIER3_TYPES.contains(&input_type) {
        web_sys::console::error_1(
            &format!(
                "Input does not support type=\"{input_type}\". Use the dedicated component instead."
            )
            .into(),
        );
    }
}
