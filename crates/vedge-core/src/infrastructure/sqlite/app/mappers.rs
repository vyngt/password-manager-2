pub mod app_setting;
pub mod extension_session;
pub mod known_device;
pub mod recent_vault;
pub mod theme;

use chrono::{DateTime, Utc};

use crate::domain::shared::{StorageError, Timestamp};

pub(crate) fn ts_to_string(ts: &Timestamp) -> String {
    ts.to_rfc3339()
}

pub(crate) fn string_to_ts(raw: &str) -> Result<Timestamp, StorageError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| StorageError::Serialization(format!("invalid timestamp {raw:?}: {e}")))
}

pub(crate) fn string_to_ts_opt(raw: Option<&str>) -> Result<Option<Timestamp>, StorageError> {
    raw.map(string_to_ts).transpose()
}

pub(crate) fn fixed_key(raw: &[u8], field: &'static str) -> Result<[u8; 32], StorageError> {
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| StorageError::Serialization(format!(
            "{field}: expected 32 bytes, got {}",
            raw.len()
        )))?;
    Ok(arr)
}
