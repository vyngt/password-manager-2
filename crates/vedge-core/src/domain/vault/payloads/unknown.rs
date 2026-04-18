use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;

/// Forward-compat preservation of entries with an unrecognized `entry_type`.
/// Kept read-only: the use-case layer refuses to re-encrypt an `UnknownPayload`
/// because we don't own its schema well enough to write it back safely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnknownPayload {
    pub meta: CommonMeta,
    /// Original JSON object as received, including the fields covered by `meta`.
    /// Preserved so a future client that understands the type can round-trip it.
    pub unknown_fields: serde_json::Value,
}
