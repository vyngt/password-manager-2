//! Recovery Key wire DTOs (slice 5.7).
//!
//! 🔴 The `recovery_slot` (`[u8;40]`) NEVER crosses this boundary — it lives only in the vault
//! file. The only recovery material on the wire is the show-once `RK1-` display (enroll
//! output) and the two user-typed documents (recover input), mirroring the create-vault /
//! emergency-kit show-once contract.

use serde::{Deserialize, Serialize};

/// The Recovery Key after enrolling — the `RK1-…` display crosses to the frontend exactly
/// once, then is dropped. The raw 32-byte key and the slot never cross.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEnrollOutputDto {
    pub recovery_key_display: String,
}

/// Recovery unlock — the two documents typed on the launch screen. The master password is
/// forgotten (that is the whole point), so it is not carried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockWithRecoveryKeyInputDto {
    pub vault_path: String,
    /// `RK1-XXXXX-…` from the Recovery Kit.
    pub recovery_key_display: String,
    /// `A3-XXXXX-…` from the Emergency Kit.
    pub secret_key_display: String,
}
