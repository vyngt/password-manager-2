//! `VaultSession` — runtime state of an open, unlocked vault.
//!
//! Built by [`UnlockVault`](crate::application::vault::use_cases::UnlockVault);
//! torn down by [`lock_vault`](crate::application::vault::use_cases::lock_vault)
//! or by going out of scope. The `Drop` impl zeroizes the KEK and the index —
//! dropping is sufficient; callers don't need to call `lock()` explicitly.
//!
//! ## Security invariants (enforced here, relied on by every use case)
//!
//! - The KEK lives in an `mlock()`-protected region for the session's lifetime.
//! - `Drop` unconditionally zeroizes both the KEK bytes and the `VaultIndex`
//!   name/URL strings — even on panic unwind.
//! - `VaultSession` is `!Clone` — an unlocked vault has exactly one owner.
//!   Shells that need multi-command access should wrap in `Arc<Mutex<…>>`.
//!
//! ## Visibility
//!
//! All fields are `pub(crate)` so the sibling `use_cases` module can read/write
//! them directly. External callers touch only the public `index()`/
//! `vault_id()`/`lock()` surface.

use std::collections::HashSet;
use std::sync::Arc;

use zeroize::{Zeroize, Zeroizing};

use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::use_cases::import_entries::ImportSession;
use crate::domain::shared::{EntryId, VaultId};
use crate::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use crate::domain::vault::entities::VaultConfig;
use crate::domain::vault::index::VaultIndex;
use crate::infrastructure::crypto::secret_mem::SecretMem;

pub struct VaultSession {
    pub(crate) vault_id: VaultId,

    /// Mlock'd on construction; zeroized on drop.
    pub(crate) kek: SecretMem<[u8; KEK_LEN]>,

    /// In-memory decrypted metadata — `Zeroize` runs during `Drop`.
    pub(crate) index: VaultIndex,

    /// Vault config as last loaded from disk. Use cases update this + the
    /// `vault_config` row together.
    pub(crate) config: VaultConfig,

    /// Injected ports. `Arc<dyn …>` so shells can share them across many
    /// sessions and so tests can swap in mocks.
    pub(crate) repo: Arc<dyn VaultRepository>,
    pub(crate) crypto: Arc<dyn CryptoProvider>,
    pub(crate) blob: Arc<dyn BlobStore>,
    pub(crate) clipboard: Arc<dyn ClipboardProvider>,

    /// Entries whose TOTP code has been revealed this session (slice 4.2). Drives
    /// audit-once-per-(session, entry): the first `reveal_totp` writes a
    /// `TotpRevealed` row, period-boundary refreshes don't. Non-secret (ids only);
    /// dropped with the session on lock, so the next unlock audits again.
    pub(crate) revealed_totp: HashSet<EntryId>,

    /// If the unlock detected a rollback (the vault's `commit_counter` was below
    /// this device's keychain baseline), the delta (baseline − file); else `None`
    /// (slice 5.2c). Non-secret, advisory: the shell reads it once right after unlock
    /// to raise a warning toast. Never blocks unlock.
    pub(crate) rollback_warning: Option<i64>,

    /// An in-progress import staging its decrypted rows in RAM (slice 5.3b).
    /// `ZeroizeOnDrop`, capacity-bounded, and **bound to this session's lifetime**:
    /// lock / TTL / cancel drops it and zeroizes the staged secrets. `None` until
    /// `begin_import` runs.
    pub(crate) import: Option<ImportSession>,

    /// One-shot capability for the forced post-recovery password change (slice 5.7).
    /// Set ONLY by `unlock_with_recovery_key` to the Secret Key the user just typed;
    /// `None` on every other unlock path. `change_password_after_recovery` `.take()`s it
    /// (errors `NoRecoveryResetPending` if absent), which both (a) gates the no-reauth
    /// password change to a session freshly opened by recovery, exactly once, and (b) feeds
    /// the SK to the forced change explicitly so it works even if the keychain re-store
    /// failed on a fresh device. Held in `Zeroizing`, so it self-zeroizes when consumed or
    /// when the session drops on any lock/TTL/cancel path.
    pub(crate) pending_recovery_reset: Option<Zeroizing<[u8; SECRET_KEY_LEN]>>,
}

impl VaultSession {
    /// `pub(crate)` constructor — the only legitimate caller is
    /// `UnlockVault` in the same application layer. No external call sites.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assemble(
        vault_id: VaultId,
        kek: SecretMem<[u8; KEK_LEN]>,
        index: VaultIndex,
        config: VaultConfig,
        repo: Arc<dyn VaultRepository>,
        crypto: Arc<dyn CryptoProvider>,
        blob: Arc<dyn BlobStore>,
        clipboard: Arc<dyn ClipboardProvider>,
    ) -> Self {
        Self {
            vault_id,
            kek,
            index,
            config,
            repo,
            crypto,
            blob,
            clipboard,
            revealed_totp: HashSet::new(),
            rollback_warning: None,
            import: None,
            pending_recovery_reset: None,
        }
    }

    /// Read-only handle to the in-memory index. UI queries go through here.
    #[must_use]
    pub const fn index(&self) -> &VaultIndex {
        &self.index
    }

    /// Identifier (resolved from the vault path) for this session.
    #[must_use]
    pub const fn vault_id(&self) -> &VaultId {
        &self.vault_id
    }

    /// The intrinsic `vault_uuid` this session was unlocked under, if set (slice 5.2.0).
    /// Always `Some` for a session produced by `UnlockVault` (unlock backfills it); the
    /// keychain call sites key the Secret Key on this, not the path.
    #[must_use]
    pub fn vault_uuid(&self) -> Option<&str> {
        self.config.vault_uuid.as_deref()
    }

    /// The rollback delta (baseline − file) if this unlock detected a rollback, else
    /// `None` (slice 5.2c). The shell reads this once post-unlock to raise a warning.
    #[must_use]
    pub const fn rollback_warning(&self) -> Option<i64> {
        self.rollback_warning
    }

    /// Whether a Recovery Key is enrolled (slice 5.7) — a derivative bool for the Settings
    /// row. The slot bytes themselves never leave core.
    #[must_use]
    pub const fn has_recovery_key(&self) -> bool {
        self.config.recovery_slot.is_some()
    }

    /// Explicit lock: consume the session and let `Drop` zeroize the KEK
    /// and `VaultIndex`. Semantically identical to letting the value go out of
    /// scope — provided as a named method so shell code reads obviously.
    #[allow(clippy::unused_self)] // consumes the session — `Drop` does the work
    pub fn lock(self) {
        // `Drop` does the work.
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        // `SecretMem::Drop` will zeroize the bytes as well; calling here
        // makes the intent explicit and handles the rare case where
        // field-drop order changes in future refactors.
        self.kek.zeroize_in_place();
        self.index.zeroize();
    }
}

// Manual `Debug` — never expose the KEK, `VaultIndex` contents, or ports.
// Logs, `.unwrap_err()` test output, and panic messages only see the vault
// id + gross index stats.
impl std::fmt::Debug for VaultSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultSession")
            .field("vault_id", &self.vault_id)
            .field("entries", &self.index.entries.len())
            .field("tags", &self.index.tags.len())
            .finish_non_exhaustive()
    }
}
