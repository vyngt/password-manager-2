use std::path::PathBuf;

use crate::domain::shared::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredVault {
    pub id: String,
    /// The vault **home** directory (`…/<name>.vedge`), slice 5.2.0.
    pub path: PathBuf,
    pub display_name: String,
    pub last_opened: Option<Timestamp>,
    pub sort_order: i32,
    /// The vault's intrinsic `vault_uuid` (slice 5.2.0). `None` for rows recorded before
    /// this slice, until their next open re-records them. Lets 5.2.2 dedup a vault by
    /// identity regardless of its on-disk path.
    pub vault_uuid: Option<String>,
}
