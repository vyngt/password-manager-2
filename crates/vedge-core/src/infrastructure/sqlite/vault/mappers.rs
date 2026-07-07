pub mod audit_log;
pub mod entry;
pub mod entry_history;
pub mod tag;
pub mod vault_config;

use chrono::{DateTime, Utc};

use crate::domain::shared::Timestamp;
use crate::domain::vault::errors::VaultError;

pub(crate) fn ts_to_string(ts: &Timestamp) -> String {
    ts.to_rfc3339()
}

pub(crate) fn string_to_ts(raw: &str) -> Result<Timestamp, VaultError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            VaultError::Storage(crate::domain::shared::StorageError::Serialization(format!(
                "invalid timestamp {raw:?}: {e}"
            )))
        })
}

pub(crate) fn string_to_ts_opt(raw: Option<&str>) -> Result<Option<Timestamp>, VaultError> {
    raw.map(string_to_ts).transpose()
}

pub(crate) fn fixed_bytes<const N: usize>(
    raw: &[u8],
    field: &'static str,
) -> Result<[u8; N], VaultError> {
    <[u8; N]>::try_from(raw).map_err(|_| VaultError::InvalidBlobLength {
        field,
        expected: N,
        actual: raw.len(),
    })
}
