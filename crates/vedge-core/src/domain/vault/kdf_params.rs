use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub alg: String,
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub version: u32,
}

impl KdfParams {
    #[must_use]
    pub fn argon2id_default() -> Self {
        Self {
            alg: "argon2id".to_owned(),
            m: 262_144,
            t: 3,
            p: 4,
            version: 1,
        }
    }

    /// **Test-only, release-absent** fast Argon2id profile (m 8 KiB / t 1 / p 1).
    /// The production KDF (256 MiB / t 3 / p 4) dominates the e2e run; this lets
    /// the harness create+unlock cheaply. Gated `#[cfg(debug_assertions)]` so it
    /// does **not** exist in a release build, and its only caller (`create_vault`)
    /// additionally requires the `VEDGE_E2E_FAST_KDF` env var — a fast KDF
    /// reachable in production would gut the vault's key-stretching.
    #[cfg(debug_assertions)]
    #[must_use]
    pub fn fast() -> Self {
        Self {
            alg: "argon2id".to_owned(),
            m: 8,
            t: 1,
            p: 1,
            version: 1,
        }
    }
}
