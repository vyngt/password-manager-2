use async_trait::async_trait;

use crate::domain::vault::errors::VaultError;

/// A k-anonymity range lookup against a breach corpus (`HaveIBeenPwned` *Pwned
/// Passwords*, slice 4.4).
///
/// The implementation receives **only** an uppercase 5-hex SHA-1 prefix — it
/// never sees a password, a full hash, a suffix, or an entry id. The caller
/// computes the SHA-1 in Rust, sends the prefix, and matches the returned
/// suffixes locally; the plaintext and the full hash never leave the process.
///
/// Error-mapping contract (infrastructure is responsible): any transport, TLS,
/// timeout, rate-limit, or parse failure maps to [`VaultError::BreachLookup`].
/// The caller degrades gracefully — a failed lookup surfaces as
/// `HealthReport::breach_check_failed` and **never fails the scan**.
#[async_trait]
pub trait BreachChecker: Send + Sync {
    /// Look up one range. `prefix` is an uppercase 5-hex SHA-1 prefix. Returns
    /// `(suffix, count)` pairs where `suffix` is the remaining uppercase 35-hex
    /// characters and `count` is the number of times that full hash appears in
    /// the corpus. Padding rows (`count == 0`) are discarded by the impl.
    async fn range(&self, prefix: &str) -> Result<Vec<(String, u32)>, VaultError>;
}
