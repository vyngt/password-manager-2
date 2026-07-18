use serde::{Deserialize, Serialize};

/// The decrypted form of a tag row. Sealed under a per-row DEK that is wrapped by the
/// KEK (slice 5.6.0), like an entry; legacy rows may still be KEK-sealed until migrated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagPayload {
    /// Always stored normalized: lowercased, trimmed, internal whitespace collapsed.
    /// Normalization is enforced at the application layer, not the domain.
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    #[serde(default)]
    pub sort_order: u32,
}
