use uuid::Uuid;

/// Generate a unique DOM id with a component-name prefix.
///
/// Use for aria-labelledby / aria-describedby / htmlFor pairings where two
/// elements need to reference each other by id. The prefix makes ids
/// self-documenting in devtools (`tooltip-…`, `dialog-title-…`).
pub fn id_with_prefix(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

/// Generate a bare UUID v4 string, no prefix. Use for stable keys on list
/// items where the id doesn't need to be visually identifiable in devtools.
pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}
