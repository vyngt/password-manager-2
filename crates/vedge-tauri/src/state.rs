//! `AppState` — the single `tauri::State<'_>` every command pulls.
//!
//! Holds two kinds of things:
//! 1. Stateless ports (crypto, KDF, keychain, clipboard, app.db repositories).
//!    Alive for the lifetime of the process.
//! 2. A registry of currently-unlocked [`VaultSession`]s, keyed by
//!    [`VaultId`]. Two-layer locking so multi-vault use doesn't serialize:
//!    the outer `std::sync::Mutex<HashMap>` is held only for the microseconds
//!    of a lookup; each session has its own `tokio::sync::Mutex`, held across
//!    use-case awaits without blocking other vaults.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex as AsyncMutex;

use vedge_core::application::app::ports::{
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository, RecentVaultRepository,
    ThemeRepository,
};
use vedge_core::application::vault::ports::{
    BiometricAuthenticator, BreachChecker, ClipboardProvider, CryptoProvider,
    KeyDerivationProvider, KeychainProvider,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{CreateVault, UnlockVault};
use vedge_core::domain::shared::{VaultId, now};

use crate::error::CommandError;

pub type SessionHandle = Arc<AsyncMutex<VaultSession>>;

/// The active-session registry value: a live session plus its hard expiry
/// deadline (slice 4.5a).
pub(crate) type SessionMap = HashMap<VaultId, SessionSlot>;

/// A registered session paired with its **absolute** hard-TTL deadline.
///
/// `expires_at == None` means the session never expires on a timer (the
/// `session_max_minutes` pref is `0`/disabled). The deadline is wall-clock
/// (`DateTime<Utc>`), deliberately **not** an `Instant`: `Instant` does not
/// advance across OS suspend on Windows, so a closed-lid laptop could otherwise
/// outlive the ceiling — exactly the case the TTL is meant to close.
pub(crate) struct SessionSlot {
    handle: SessionHandle,
    expires_at: Option<DateTime<Utc>>,
}

impl SessionSlot {
    /// Whether this slot's deadline has passed as of `at`. `at` is a parameter
    /// (not a live `now()`) so expiry is unit-testable without mocking the clock.
    fn is_expired(&self, at: DateTime<Utc>) -> bool {
        deadline_passed(self.expires_at, at)
    }
}

/// Pure deadline comparison: `None` never expires; otherwise `at >= deadline`.
/// Wall-clock (`DateTime`) — so a suspend/resume that jumps the clock forward
/// crosses the deadline, unlike a monotonic `Instant`.
fn deadline_passed(deadline: Option<DateTime<Utc>>, at: DateTime<Utc>) -> bool {
    deadline.is_some_and(|d| at >= d)
}

/// Turn an optional TTL into an absolute deadline. `None` ⇒ never expires;
/// `checked_add_signed` avoids a panic on an absurd (overflowing) duration.
fn deadline_from_ttl(ttl: Option<Duration>) -> Option<DateTime<Utc>> {
    let signed = chrono::Duration::from_std(ttl?).ok()?;
    now().checked_add_signed(signed)
}

/// Remove every session whose hard deadline has passed, returning the handles
/// so the reaper can audit-then-drop (see [`crate::scheduler`]). The outer map
/// lock is held only for the drain — never across the audit await.
pub(crate) fn drain_expired_sessions(sessions: &StdMutex<SessionMap>) -> Vec<SessionHandle> {
    let at = now();
    let Ok(mut guard) = sessions.lock() else {
        return Vec::new();
    };
    // Collect expired keys first: the `iter()` borrow must end before `remove()`
    // takes a mutable borrow of the same map (so the collect is NOT needless).
    #[allow(clippy::needless_collect)]
    let expired: Vec<VaultId> = guard
        .iter()
        .filter(|(_, slot)| slot.is_expired(at))
        .map(|(id, _)| id.clone())
        .collect();
    expired
        .into_iter()
        .filter_map(|id| guard.remove(&id).map(|slot| slot.handle))
        .collect()
}

pub struct AppState {
    // ---- stateless, process-lifetime ports ---------------------------------
    pub crypto: Arc<dyn CryptoProvider>,
    pub kdf: Arc<dyn KeyDerivationProvider>,
    pub keychain: Arc<dyn KeychainProvider>,
    pub biometric: Arc<dyn BiometricAuthenticator>,
    pub clipboard: Arc<dyn ClipboardProvider>,
    /// `HaveIBeenPwned` k-anonymity checker (slice 4.4). Always present; the health
    /// command only hands it to the scan when the user has opted in.
    pub breach: Arc<dyn BreachChecker>,

    // ---- app.db repositories (one per repo, sharing one DB connection) -----
    pub recent_vaults: Arc<dyn RecentVaultRepository>,
    pub app_settings: Arc<dyn AppSettingRepository>,
    pub themes: Arc<dyn ThemeRepository>,
    pub known_devices: Arc<dyn KnownDeviceRepository>,
    pub extension_sessions: Arc<dyn ExtensionSessionRepository>,

    // ---- pre-wired vault-lifecycle use cases (per-vault infra built via
    //      factories inside `execute`; constructed once in `compose`) ---------
    pub unlock_vault: UnlockVault,
    pub create_vault: CreateVault,

    // ---- active vault sessions ---------------------------------------------
    sessions: Arc<StdMutex<SessionMap>>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        crypto: Arc<dyn CryptoProvider>,
        kdf: Arc<dyn KeyDerivationProvider>,
        keychain: Arc<dyn KeychainProvider>,
        biometric: Arc<dyn BiometricAuthenticator>,
        clipboard: Arc<dyn ClipboardProvider>,
        breach: Arc<dyn BreachChecker>,
        recent_vaults: Arc<dyn RecentVaultRepository>,
        app_settings: Arc<dyn AppSettingRepository>,
        themes: Arc<dyn ThemeRepository>,
        known_devices: Arc<dyn KnownDeviceRepository>,
        extension_sessions: Arc<dyn ExtensionSessionRepository>,
        unlock_vault: UnlockVault,
        create_vault: CreateVault,
    ) -> Self {
        Self {
            crypto,
            kdf,
            keychain,
            biometric,
            clipboard,
            breach,
            recent_vaults,
            app_settings,
            themes,
            known_devices,
            extension_sessions,
            unlock_vault,
            create_vault,
            sessions: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    /// Look up an unlocked session. Returns `VaultNotOpen` when the vault
    /// isn't in the registry, or `SessionExpired` when its hard TTL has
    /// elapsed — in which case the slot is **evicted here** (the check is at
    /// entry, not at use: a command that starts valid and runs for seconds
    /// completes on a now-expired session, which is honest). Eviction here is
    /// audit-silent; the reaper / `lock_vault` normally land the `Locked` row
    /// first (slice 4.5a).
    pub fn get_session(&self, id: &VaultId) -> Result<SessionHandle, CommandError> {
        let Ok(mut guard) = self.sessions.lock() else {
            return Err(CommandError::Internal);
        };
        // Read the handle + expiry as owned values, ending the borrow before
        // the possible `remove` below (avoids a get/remove borrow conflict).
        match guard
            .get(id)
            .map(|slot| (Arc::clone(&slot.handle), slot.is_expired(now())))
        {
            None => Err(CommandError::VaultNotOpen(id.path().display().to_string())),
            Some((_, true)) => {
                guard.remove(id);
                Err(CommandError::SessionExpired)
            }
            Some((handle, false)) => Ok(handle),
        }
    }

    /// `true` if the vault is currently unlocked **and** its hard TTL has not
    /// elapsed. Must stay TTL-aware: this is a path separate from
    /// `get_session`, and the e2e suite asserts lock state exclusively through
    /// it (via the `is_unlocked` command).
    #[must_use]
    pub fn is_unlocked(&self, id: &VaultId) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|g| g.get(id).map(|slot| !slot.is_expired(now())))
            .unwrap_or(false)
    }

    /// Register a new session with an optional hard TTL. `ttl == None` ⇒ the
    /// session never expires on a timer (pref disabled). Returns `Invalid` if
    /// the vault was already unlocked — caller must lock first. The outer mutex
    /// is held only for the `HashMap` insert; it never wraps an await.
    pub fn insert_session(
        &self,
        id: VaultId,
        session: VaultSession,
        ttl: Option<Duration>,
    ) -> Result<(), CommandError> {
        let Ok(mut guard) = self.sessions.lock() else {
            return Err(CommandError::Internal);
        };
        // Reject a genuine double-unlock, but OVERWRITE a stale expired slot: an
        // expired-but-not-yet-reaped session (evicted only by the reaper / the
        // frontend poll / a `get_session`) must not block a legitimate re-unlock —
        // `is_unlocked` already reports it as locked. The replaced session's KEK
        // zeroizes when its last `Arc` clone drops.
        if guard.get(&id).is_some_and(|slot| !slot.is_expired(now())) {
            return Err(CommandError::Invalid(format!(
                "vault already unlocked: {}",
                id.path().display()
            )));
        }
        guard.insert(
            id,
            SessionSlot {
                handle: Arc::new(AsyncMutex::new(session)),
                expires_at: deadline_from_ttl(ttl),
            },
        );
        Ok(())
    }

    /// Re-stamp an existing session's deadline (slice 4.5a, Decision 6). Called
    /// after a re-authentication that keeps the session in place —
    /// `change_password` — so a just-proved master credential doesn't get
    /// reaped out from under the user. A no-op if the vault isn't open.
    pub fn touch_deadline(&self, id: &VaultId, ttl: Option<Duration>) {
        if let Ok(mut guard) = self.sessions.lock() {
            if let Some(slot) = guard.get_mut(id) {
                slot.expires_at = deadline_from_ttl(ttl);
            }
        }
    }

    /// Remove a session from the registry. Returns the handle for the caller
    /// to drop (or to audit-then-drop via [`record_lock`] — the underlying
    /// session zeroizes via `Drop` when the last `Arc` clone releases).
    ///
    /// [`record_lock`]: vedge_core::application::vault::use_cases::record_lock
    #[must_use]
    pub fn remove_session(&self, id: &VaultId) -> Option<SessionHandle> {
        self.sessions
            .lock()
            .ok()?
            .remove(id)
            .map(|slot| slot.handle)
    }

    /// A clone of the session-registry handle for the background scheduler
    /// (slice 4.5a), which sweeps expired slots without needing the whole
    /// `AppState`. The map stays private; only the reaper reaches it this way.
    pub(crate) fn sessions_handle(&self) -> Arc<StdMutex<SessionMap>> {
        Arc::clone(&self.sessions)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::needless_pass_by_value
    )]

    use super::*;
    use crate::test_support::{
        fast_params, state_fixture, unlocked_vault, unlocked_vault_with_ttl,
    };
    use std::path::PathBuf;
    use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};

    #[tokio::test]
    async fn get_session_on_unknown_vault_returns_not_open() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        let id = VaultId::new(PathBuf::from("/tmp/nope.vdb"));
        match state.get_session(&id) {
            Err(CommandError::VaultNotOpen(_)) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn is_unlocked_flips_on_insert_and_remove() {
        // We can't easily construct a real VaultSession here without running
        // UnlockVault end-to-end. The is_unlocked contract is trivial —
        // it's a HashMap::contains_key — so we defer this to an integration
        // test in a later phase (tests/vault_commands.rs).
        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        assert!(!state.is_unlocked(&VaultId::new(PathBuf::from("/x"))));
    }

    #[tokio::test]
    async fn create_vault_leaves_vault_unlocked() {
        use vedge_core::CreateVaultInput;
        use zeroize::Zeroizing;

        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        let vault_path = dir.path().join("new.vdb");
        let vault_id = VaultId::new(vault_path.clone());

        // Mirror the command body: run the use case, then register the session.
        let out = state
            .create_vault
            .execute(CreateVaultInput {
                vault_path,
                master_password: Zeroizing::new("correct horse battery staple".into()),
                secret_key: None,
                kdf_params: Some(fast_params()),
            })
            .await
            .unwrap();
        assert!(out.keychain_stored);
        state
            .insert_session(vault_id.clone(), out.session, None)
            .unwrap();

        // The vault ends UNLOCKED with a live, queryable session — no unlock.
        assert!(state.is_unlocked(&vault_id));
        let handle = state.get_session(&vault_id).unwrap();
        assert!(handle.lock().await.index().all_active().is_empty());
    }

    #[tokio::test]
    async fn create_vault_on_existing_path_is_already_exists() {
        use vedge_core::CreateVaultInput;
        use vedge_core::domain::vault::errors::VaultError;
        use zeroize::Zeroizing;

        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        let vault_path = dir.path().join("dup.vdb");

        let mk_input = || CreateVaultInput {
            vault_path: vault_path.clone(),
            master_password: Zeroizing::new("pw".into()),
            secret_key: None,
            kdf_params: Some(fast_params()),
        };

        // First create succeeds; drop the session to release the file handle.
        drop(
            state
                .create_vault
                .execute(mk_input())
                .await
                .unwrap()
                .session,
        );

        // Second create on the same path → VaultAlreadyExists → AlreadyExists.
        let err = state.create_vault.execute(mk_input()).await.unwrap_err();
        assert!(matches!(err, VaultError::VaultAlreadyExists));
        assert!(matches!(
            CommandError::from(VaultError::VaultAlreadyExists),
            CommandError::AlreadyExists
        ));
    }

    // ---- hard session TTL (slice 4.5a) --------------------------------------

    #[test]
    fn deadline_from_ttl_none_never_expires() {
        assert!(deadline_from_ttl(None).is_none());
        assert!(!deadline_passed(None, now()));
    }

    #[test]
    fn ttl_uses_wall_clock_not_instant() {
        // A hard TTL is an absolute wall-clock deadline: a clock that jumps
        // forward past it (resume-from-suspend) expires the session — which a
        // monotonic `Instant` would miss.
        let deadline = deadline_from_ttl(Some(Duration::from_secs(60))).unwrap();
        assert!(!deadline_passed(
            Some(deadline),
            deadline - chrono::Duration::seconds(1)
        ));
        assert!(deadline_passed(
            Some(deadline),
            deadline + chrono::Duration::seconds(1)
        ));
    }

    #[tokio::test]
    async fn get_session_expired_evicts_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault_with_ttl(&dir, "v.vdb", Some(Duration::ZERO)).await;

        match state.get_session(&vault_id) {
            Err(CommandError::SessionExpired) => {}
            other => panic!("expected SessionExpired, got {other:?}"),
        }
        // Evicted on that lookup: a second lookup now reports not-open.
        assert!(!state.is_unlocked(&vault_id));
        match state.get_session(&vault_id) {
            Err(CommandError::VaultNotOpen(_)) => {}
            other => panic!("expected VaultNotOpen after eviction, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn is_unlocked_is_ttl_aware() {
        // The e2e suite asserts lock state exclusively through `is_unlocked`, so
        // it MUST report an expired session as locked.
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault_with_ttl(&dir, "v.vdb", Some(Duration::ZERO)).await;
        assert!(!state.is_unlocked(&vault_id));
    }

    #[tokio::test]
    async fn session_max_zero_disables_expiry() {
        // `ttl == None` models `session_max_minutes = 0` — never expires.
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault_with_ttl(&dir, "v.vdb", None).await;
        assert!(state.is_unlocked(&vault_id));
        assert!(state.get_session(&vault_id).is_ok());
    }

    #[tokio::test]
    async fn touch_deadline_resets_expiry() {
        // Simulates re-auth (`change_password`): after a lapsed deadline, a
        // re-stamp pushes it out and the session stays alive (Decision 6).
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault_with_ttl(&dir, "v.vdb", Some(Duration::ZERO)).await;
        // `is_unlocked` reports expired but does NOT evict (only `get_session`
        // does), so the slot survives for the re-stamp below.
        assert!(!state.is_unlocked(&vault_id));
        state.touch_deadline(&vault_id, Some(Duration::from_secs(3600)));
        assert!(
            state.is_unlocked(&vault_id),
            "re-stamped deadline keeps it unlocked"
        );
    }

    #[tokio::test]
    async fn insert_session_overwrites_expired_but_rejects_live() {
        use vedge_core::CreateVaultInput;
        use zeroize::Zeroizing;

        // Start with an EXPIRED-but-unreaped slot for `vault_id`.
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault_with_ttl(&dir, "v.vdb", Some(Duration::ZERO)).await;
        assert!(!state.is_unlocked(&vault_id));

        // A fresh live session registered under the same id must OVERWRITE the
        // stale expired slot (a re-unlock during the expiry-before-reap window
        // must succeed, not fail "already unlocked"). Build the session from a
        // distinct on-disk vault so there's no file contention with the first.
        let session2 = state
            .create_vault
            .execute(CreateVaultInput {
                vault_path: dir.path().join("v2.vdb"),
                master_password: Zeroizing::new("pw2".into()),
                secret_key: None,
                kdf_params: Some(fast_params()),
            })
            .await
            .unwrap()
            .session;
        state
            .insert_session(vault_id.clone(), session2, None)
            .expect("re-unlock over an expired slot must be accepted");
        assert!(state.is_unlocked(&vault_id));

        // Now the slot is LIVE — a genuine double-unlock is still rejected.
        let session3 = state
            .create_vault
            .execute(CreateVaultInput {
                vault_path: dir.path().join("v3.vdb"),
                master_password: Zeroizing::new("pw3".into()),
                secret_key: None,
                kdf_params: Some(fast_params()),
            })
            .await
            .unwrap()
            .session;
        assert!(matches!(
            state.insert_session(vault_id.clone(), session3, None),
            Err(CommandError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn reaper_audits_locked_even_under_contention() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;

        // A slow in-flight command still holds a clone of the handle. The OLD
        // `lock_vault` (`Arc::try_unwrap` × 256) would fail to unwrap under this
        // and silently DROP the `Locked` row; the new audit-then-drop path must
        // still land it.
        let contender = state.get_session(&vault_id).unwrap();

        // Expire the session, then run the reaper's drain + audit.
        state.touch_deadline(&vault_id, Some(Duration::ZERO));
        let mut handles = drain_expired_sessions(&state.sessions_handle());
        assert_eq!(handles.len(), 1, "the expired slot should be drained");
        let handle = handles.pop().unwrap();

        let guard = handle.lock().await;
        vedge_core::record_lock(&guard).await.unwrap();

        // The Locked row landed despite the outstanding `contender` clone.
        let page = vedge_core::list_audit(
            &guard,
            AuditQuery {
                actions: vec![AuditAction::Locked],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            page.total >= 1,
            "the Locked audit row must land under contention"
        );

        drop(guard);
        drop(handle);
        drop(contender);
        assert!(!state.is_unlocked(&vault_id));
    }
}
