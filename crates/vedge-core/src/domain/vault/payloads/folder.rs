use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,
}
