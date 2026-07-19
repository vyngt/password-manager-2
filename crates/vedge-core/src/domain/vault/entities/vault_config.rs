use crate::domain::shared::Timestamp;
use crate::domain::vault::kdf_params::KdfParams;

/// The vault schema version this build understands.
///
/// Restore and import refuse a backup whose `schema_version` exceeds this, and unlock
/// refuses a live vault above it (slice 5.6.0) — so an older build fails cleanly instead
/// of mysteriously at decrypt time. Bump when a build produces at-rest data a prior build
/// cannot read. Note the trigger is the DATA semantics, not the column: an additive nullable
/// column alone does NOT bump (`vault_uuid` in 4.6a, `commit_counter` in 5.2c did not) — but
/// 5.6.0's additive `tags.dek_wrapped` DOES, because a migrated tag's ciphertext moves from
/// KEK-sealed to DEK-sealed and a v1 build would fail to decrypt it.
pub const CURRENT_SCHEMA_VERSION: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultConfig {
    pub id: String,
    pub magic: String,
    pub schema_version: i32,
    pub vault_salt: [u8; 32],
    pub kdf_params: KdfParams,
    pub verify_hash: [u8; 32],
    pub preferred_cipher_suite: i32,
    pub trash_retention_days: i32,
    pub audit_retention_days: i32,
    pub created_at: Timestamp,
    pub last_unlocked_at: Option<Timestamp>,
    /// Intrinsic vault identity (ULID) — answers "which *vault* is this?", distinct
    /// from `VaultId(PathBuf)` ("which *file* is open?"). Minted on create, backfilled
    /// on first open of a pre-4.6 vault. `None` only for a not-yet-backfilled vault.
    /// A *copy* of a vault is the same vault (same uuid) — a future "duplicate as a new
    /// vault" feature must mint a fresh one. Slice 4.6a; see arch note 13 - Vault Identity.
    pub vault_uuid: Option<String>,
    /// Monotonic write counter, bumped by the repository on every content mutation.
    /// Mirrored into the OS keychain (keyed on `vault_uuid`) and compared at unlock:
    /// a file counter *below* the keychain baseline signals a rollback. Advisory only
    /// — a mismatch warns, never blocks unlock. Slice 5.2c; see 06 - Rollback Detection.
    pub commit_counter: i64,
    /// Optional override of the default `<home>/snapshots` store location (slice 5.2.1).
    /// `None` ⇒ the default. Read at unlock; never key material.
    pub backup_dir: Option<String>,
    /// Opt-in snapshot retention (Decision ⑯). `None` ⇒ keep everything (the default —
    /// a password manager must never silently delete a user's history). When set, the
    /// maintenance prune keeps this many, floored at 3, never the newest, never a
    /// `pre-restore` or stale-credential snapshot.
    pub backup_keep_count: Option<i32>,
    /// When a snapshot was last taken. Written by a targeted repo update on
    /// `create_snapshot`, deliberately kept out of the config-upsert `update_columns`
    /// (the `commit_counter` precedent) so a stale `save_config` cannot clobber it.
    pub last_snapshot_at: Option<Timestamp>,
    /// When a `.vbk` was last written (by `backup_vault`). 🔴 SEPARATE from
    /// `last_snapshot_at` (Decision ⑧): a snapshot is NOT a backup. The credential-health
    /// card reads ONLY this — a directory full of snapshots must never silence
    /// "you have never backed up this vault".
    pub last_backup_at: Option<Timestamp>,
    /// The Recovery Key slot (slice 5.7): `AES-KW(wrap_key = KEK_rec, payload = KEK)`,
    /// 40 bytes. `Some` ⇒ recovery is enrolled; `None` ⇒ off. Additive nullable, NO
    /// `schema_version` bump (the `vault_uuid`/`commit_counter` precedent).
    ///
    /// 🔴 Write discipline: set/cleared ONLY by the targeted `set_recovery_slot` /
    /// `clear_recovery_slot` repo methods and OMITTED from the `save_config`
    /// `update_columns` (like `commit_counter`), so a generic unlock-time `save_config`
    /// can never clobber or resurrect it. It IS in `rewrap_all_deks`'s `update_columns`,
    /// so a KEK change nulls it in the SAME transaction as the DEK rewrap (a stale slot
    /// would unwrap a dead KEK — a silent brick at recovery time). Never key material that
    /// crosses to WASM — it lives only here, in the vault file.
    pub recovery_slot: Option<[u8; 40]>,
    /// When the master password's KEK was last refreshed (slice 5.9 ③). Written by every
    /// credential path that re-derives the password KEK — `change_password`,
    /// `rotate_secret_key`, `rekey_vault`, `change_password_after_recovery` — in the SAME
    /// transaction as the rewrap. `None` ⇒ unknown → the credential-age nudge treats it as
    /// `created_at`, never "never". Additive nullable, NO `schema_version` bump (the
    /// `recovery_slot`/`vault_uuid` precedent). Same write discipline as `recovery_slot`:
    /// in `rewrap_all_deks`'s `update_columns` + targeted setters, OMITTED from
    /// `save_config` so a generic unlock can't clobber it.
    pub last_password_change_at: Option<Timestamp>,
    /// When the Secret Key was last rotated (slice 5.9 ③). Written ONLY when the SK
    /// actually changes — `rotate_secret_key` and `rekey_vault` with a new SK — NOT by a
    /// recovery reset (which re-stores the SAME key). Keyed off the explicit
    /// `secret_key_rotated` flag on `ChangePasswordInput`, never `new_secret_key.is_some()`
    /// (recovery-reset passes `Some(same_key)` and would be wrongly stamped). `None` ⇒ never
    /// rotated → the nudge treats it as `created_at`. Same additive-nullable, no-bump,
    /// write-discipline story as `last_password_change_at`.
    pub last_secret_key_rotation_at: Option<Timestamp>,
}
