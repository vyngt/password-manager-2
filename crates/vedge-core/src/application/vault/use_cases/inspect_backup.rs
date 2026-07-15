//! Preview a `.vbk` before acting on it (slice 5.2b; two-mode since 5.2.2) — no session,
//! no KEK, no writes.
//!
//! **ONE preview, two callers.** *Open backup* and *Advanced ▸ Replace* ask different
//! questions of the same archive, and the difference is entirely in what is supplied:
//!
//! | | `target_vault` | `dest_home` | asks |
//! |---|---|---|---|
//! | **Open backup** | `None` | `Some` | is the destination free? is this a duplicate? |
//! | **Replace** | `Some` | `None` | does it match this vault? would it roll back? |
//!
//! Both ask M3's question — *"will this even open with the credentials I have?"* — and both
//! get it answered before the user commits ten minutes to finding out the hard way.
//!
//! Every field the caller did not ask about is `None`/`false`, and the hard-stops it computes
//! are exactly the refusals [`super::open_backup`] and [`super::replace_vault`] enforce — so
//! the UI can render a *blocked* dialog instead of discovering a refusal on submit.
//!
//! 🔴 This is a PREVIEW, never a gate. Both use cases re-check everything themselves: the
//! disk can change between the preview and the click, and a programmatic caller can skip the
//! preview entirely.

use std::path::PathBuf;

use tracing::instrument;

use crate::domain::vault::entities::CURRENT_SCHEMA_VERSION;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive;
use crate::infrastructure::backup::manifest::BACKUP_FORMAT_VERSION;
use crate::infrastructure::backup::target::{TargetState, read_target_state};

use super::open_backup::find_duplicate;

/// Which archive to preview, and in which mode. See the module docs for the two shapes.
#[derive(Debug, Clone)]
pub struct InspectBackupInput {
    pub archive_path: PathBuf,
    /// **Replace mode.** The live vault that would be overwritten.
    pub target_vault: Option<PathBuf>,
    /// **Open mode.** Where the vault would be created — previewed so the UI can block on an
    /// occupied destination instead of erroring on submit.
    pub dest_home: Option<PathBuf>,
    /// Every vault home this machine knows about (from the shell's `app.db` recents), for the
    /// ② duplicate check.
    pub known_vaults: Vec<PathBuf>,
}

/// What the target of a Replace actually is. Mirrors
/// [`crate::infrastructure::backup::target::TargetState`] as a flat, wire-friendly value.
///
/// 🔴 `Missing` and `Unreadable` are NOT the same answer, and collapsing them is finding H2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetStateKind {
    Missing,
    Unreadable,
    Readable,
}

/// What acting on this backup would do — backup facts, target facts, and the computed
/// hard-stops. Derivatives only; no key material.
// The bools are independent refusal reasons, each mapping to its own message; a single enum
// could not express "unknown format AND an occupied destination".
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct BackupPreview {
    // ---- the backup ----
    pub format_version: u32,
    pub backup_vault_uuid: Option<String>,
    pub backup_schema_version: i32,
    pub created_at: String,
    pub backup_entry_count: u64,
    pub backup_blob_count: u64,

    // ---- the current target (Replace mode) ----
    pub target_state: TargetStateKind,
    pub target_uuid: Option<String>,
    /// 🔴 `Option`, never a bare count. **Never render `0` for an unknown entry count** — a
    /// zero the user believes is a zero the user acts on.
    pub target_entry_count: Option<u64>,
    pub target_last_unlocked_at: Option<String>,
    pub target_commit_counter: Option<i64>,

    // ---- hard stops ----
    /// Replace: the backup belongs to a different vault. A HARD refusal — no confirm.
    pub uuid_mismatch: bool,
    pub unknown_format: bool,
    pub unknown_schema: bool,
    /// Open: something already exists at `dest_home`. A HARD refusal — no override.
    pub dest_occupied: bool,

    // ---- risks, each needing its OWN acknowledgement ----
    /// Replace: restoring to state N while this device is at M — the delta it would drop.
    pub rollback_delta: Option<i64>,
    /// **M3.** `Some(true)` ⇒ the backup was sealed with a different master password / Secret
    /// Key, so it needs the ones in force on `created_at` — including the `Emergency Kit`
    /// printed at that time.
    ///
    /// 🔴 `None` means **unknown**, and must be shown as unknown — *"`VEdge` can't tell whether
    /// it opens with your current password and Secret Key."* It is never `Some(false)` unless
    /// both prefixes were read and genuinely matched. **Never assert a match you did not
    /// verify.**
    pub credentials_differ: Option<bool>,
    /// **②.** A live, openable vault on this machine already carries this backup's uuid, so
    /// opening it is making a COPY — and the copy will be given a fresh identity of its own.
    pub duplicate_of: Option<PathBuf>,
}

#[instrument(skip_all, fields(archive = %input.archive_path.display()))]
pub async fn inspect_backup(input: InspectBackupInput) -> Result<BackupPreview, VaultError> {
    // `read_manifest`, not `verify_archive`: a preview is cheap by design and must not stream
    // a multi-GB archive to render a dialog. The real integrity gate runs in the use case that
    // actually touches the disk.
    let manifest = archive::read_manifest(&input.archive_path)?;

    // 🔴 H1: refuse NEWER, accept OLDER — must mirror the use cases exactly, or the preview
    // would block a backup the core would happily read (or vice versa).
    let unknown_format = manifest.format_version > BACKUP_FORMAT_VERSION;
    let unknown_schema = manifest.schema_version > CURRENT_SCHEMA_VERSION;

    // ---- Open mode: is the destination free, and is this a duplicate? ----
    let dest_occupied = input.dest_home.as_ref().is_some_and(|d| d.exists());

    // 🔴 ② is an OPEN-mode question ONLY. On the Replace path the target *is* the same vault —
    // that is the whole precondition — so scanning for a "duplicate" would find the target itself
    // and the UI would cheerfully tell the user they are making a separate copy while they are in
    // fact overwriting the original. Two verbs, two questions.
    //
    // Reuses `open_backup`'s scan (not a second copy of it) so the preview cannot promise an
    // outcome the use case will not deliver — and it keeps the identity it read, so M3's
    // credential compare below does not have to open the same database a second time.
    let duplicate = if input.target_vault.is_none() {
        find_duplicate(&input.known_vaults, manifest.vault_uuid.as_deref()).await
    } else {
        None
    };
    let duplicate_of = duplicate.as_ref().map(|(home, _)| home.clone());

    // ---- Replace mode: what is actually at the target? ----
    let state = match input.target_vault.as_ref() {
        Some(t) => read_target_state(t).await,
        None => TargetState::Missing,
    };
    let (target_state, target) = match state {
        TargetState::Missing => (TargetStateKind::Missing, None),
        TargetState::Unreadable => (TargetStateKind::Unreadable, None),
        TargetState::Readable(id) => (TargetStateKind::Readable, Some(id)),
    };

    let target_uuid = target.as_ref().and_then(|t| t.vault_uuid.clone());
    let target_entry_count = target.as_ref().map(|t| t.entry_count);
    let target_last_unlocked_at = target.as_ref().and_then(|t| t.last_unlocked_at.clone());
    let target_commit_counter = target.as_ref().and_then(|t| t.commit_counter);

    let uuid_mismatch = match (manifest.vault_uuid.as_deref(), target_uuid.as_deref()) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };

    // Restoring this backup takes the vault back to the backup's `commit_counter`. If the live
    // target is AHEAD, the delta is how much state it would drop. Only computable when the
    // target's counter is readable AND strictly greater.
    let rollback_delta = match target_commit_counter {
        Some(t) if t > manifest.commit_counter => Some(t.saturating_sub(manifest.commit_counter)),
        _ => None,
    };

    // ---- M3: compare the backup's credentials against whatever we have to compare with ----
    // In Replace mode that is the target. In Open mode there IS no target — but if this is a
    // duplicate, the live original is the honest comparand, and the only one: it is the vault
    // whose password the user is about to try.
    let live_prefix = target.as_ref().map_or_else(
        // Open mode: already read during the duplicate scan — no second open of the same DB.
        || {
            duplicate
                .as_ref()
                .and_then(|(_, id)| id.verify_hash_prefix.clone())
        },
        |t| t.verify_hash_prefix.clone(),
    );
    // 🔴 Both sides must be known. `None` on either ⇒ `None` ⇒ "unknown", NEVER `Some(false)`.
    let credentials_differ = match (
        manifest.verify_hash_prefix.as_deref(),
        live_prefix.as_deref(),
    ) {
        (Some(backup), Some(live)) => Some(backup != live),
        _ => None,
    };

    Ok(BackupPreview {
        format_version: manifest.format_version,
        backup_vault_uuid: manifest.vault_uuid,
        backup_schema_version: manifest.schema_version,
        created_at: manifest.created_at,
        backup_entry_count: manifest.entry_count,
        backup_blob_count: manifest.blob_count,
        target_state,
        target_uuid,
        target_entry_count,
        target_last_unlocked_at,
        target_commit_counter,
        uuid_mismatch,
        unknown_format,
        unknown_schema,
        dest_occupied,
        rollback_delta,
        credentials_differ,
        duplicate_of,
    })
}
