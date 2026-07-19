//! Slice 5.9 ③ — resolved credential ages for the Credentials-tab nudge.
//!
//! The ages come from `vault_config` (`last_password_change_at` /
//! `last_secret_key_rotation_at`), NOT from the retention-pruned audit log — a derived age would
//! silently read "never" once the last `PasswordChanged` row is pruned, which is exactly when the
//! credential is oldest (the inverted failure mode). A `NULL` (never explicitly changed) resolves
//! to the vault's `created_at`, so the nudge always measures a real age, never "never".

use crate::application::vault::session::VaultSession;
use crate::domain::shared::Timestamp;
use crate::domain::vault::entities::VaultConfig;

/// The vault's credential ages, RESOLVED so a never-changed credential reads as the vault's
/// creation time rather than a null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialStatus {
    /// When the master password's KEK was last refreshed, or `created_at` if never.
    pub password_changed_at: Timestamp,
    /// When the Secret Key was last rotated, or `created_at` if never.
    pub secret_key_rotated_at: Timestamp,
}

/// Read the resolved credential ages off the unlocked session's config.
#[must_use]
pub fn credential_status(session: &VaultSession) -> CredentialStatus {
    resolve(&session.config)
}

/// `NULL → created_at`. Kept separate from [`credential_status`] so it is unit-testable without a
/// full session.
fn resolve(config: &VaultConfig) -> CredentialStatus {
    CredentialStatus {
        password_changed_at: config.last_password_change_at.unwrap_or(config.created_at),
        secret_key_rotated_at: config
            .last_secret_key_rotation_at
            .unwrap_or(config.created_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::now;
    use crate::domain::vault::kdf_params::KdfParams;

    fn config(created_at: Timestamp) -> VaultConfig {
        VaultConfig {
            id: "default".to_owned(),
            magic: "VEDG".to_owned(),
            schema_version: 2,
            vault_salt: [0u8; 32],
            kdf_params: KdfParams::argon2id_default(),
            verify_hash: [0u8; 32],
            preferred_cipher_suite: 1,
            trash_retention_days: 30,
            audit_retention_days: 90,
            created_at,
            last_unlocked_at: None,
            vault_uuid: None,
            commit_counter: 0,
            backup_dir: None,
            backup_keep_count: None,
            last_snapshot_at: None,
            last_backup_at: None,
            recovery_slot: None,
            last_password_change_at: None,
            last_secret_key_rotation_at: None,
        }
    }

    #[test]
    fn null_ages_resolve_to_created_at_never_never() {
        let created = now();
        let status = resolve(&config(created));
        assert_eq!(status.password_changed_at, created);
        assert_eq!(status.secret_key_rotated_at, created);
    }

    #[test]
    fn set_ages_pass_through() {
        let created = now();
        let changed = created + chrono::Duration::days(400);
        let rotated = created + chrono::Duration::days(800);
        let mut c = config(created);
        c.last_password_change_at = Some(changed);
        c.last_secret_key_rotation_at = Some(rotated);
        let status = resolve(&c);
        assert_eq!(status.password_changed_at, changed);
        assert_eq!(status.secret_key_rotated_at, rotated);
    }
}
