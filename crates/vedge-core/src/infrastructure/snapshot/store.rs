//! The content-addressed snapshot store (slice 5.2.1) — pure filesystem, no DB, no session.
//!
//! Layout inside a vault home's `snapshots/` directory:
//! ```text
//! snapshots/
//!   objects/
//!     b3-<64hex>.blob        ← immutable, content-addressed; the filename IS the hash
//!   <dir-name>/
//!     vault.vdb              ← the VACUUM INTO snapshot
//!     manifest.json          ← written LAST; a dir without it is an incomplete (crashed) create
//! ```
//!
//! Garbage collection is **mark-and-sweep** (Decision ⑪) — *never a refcount*. The set of
//! reachable objects is the union of every complete snapshot's manifest object hashes;
//! anything else in the pool is an orphan (a crashed create, or a deleted snapshot's now-
//! unshared blob) and is swept. A leaked orphan is harmless; deleting a *referenced* object
//! is data loss — so the sweep only ever deletes what no manifest names.
//!
//! Objects are addressed by the hash of their **ciphertext** (`nonce || ciphertext` on
//! disk), never the plaintext (Decision ⑫ — convergent encryption rejected). This dedups the
//! same blob across snapshots and nothing more.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::shared::{StorageError, Timestamp};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive::hash_file;

use super::manifest::{
    OBJECT_EXT, OBJECT_PREFIX, OBJECTS_DIR, SNAPSHOT_MANIFEST_NAME, SnapshotManifest,
};

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

/// The object pool inside a snapshots store: `<snapshots>/objects`.
#[must_use]
pub fn objects_dir(snapshots_dir: &Path) -> PathBuf {
    snapshots_dir.join(OBJECTS_DIR)
}

/// `objects/b3-<64hex>.blob`. The filename IS the content hash — verification needs no
/// manifest lookup, and bit rot is detectable per-object.
#[must_use]
pub fn object_path(snapshots_dir: &Path, blake3_hex: &str) -> PathBuf {
    objects_dir(snapshots_dir).join(format!("{OBJECT_PREFIX}{blake3_hex}.{OBJECT_EXT}"))
}

/// A snapshot directory's name from its timestamp.
///
/// ISO-8601 **basic**, millis, `Z`-suffixed (`20260714T153012004Z`): no `:` (Windows forbids
/// it), no locale, no DST hole. Lexicographic order == chronological order.
#[must_use]
pub fn snapshot_dir_name(at: Timestamp) -> String {
    at.format("%Y%m%dT%H%M%S%3fZ").to_string()
}

/// The staging directory a snapshot is built in before its atomic rename into place.
///
/// `<snapshots>/.<name>.tmp` — dot-prefixed so it never collides with a real snapshot and is
/// obviously transient; reaped by [`sweep_objects`] if a create crashes.
#[must_use]
pub fn staging_dir(snapshots_dir: &Path, name: &str) -> PathBuf {
    snapshots_dir.join(format!(".{name}.tmp"))
}

/// Copy a blob into the pool if absent, addressed by the BLAKE3 of its on-disk ciphertext.
///
/// Idempotent: an already-pooled object is a no-op. tmp → rename-to-final-name, so a crash
/// never leaves a truncated object under a valid name. Returns `(content_hash, size)`.
pub fn put_object(snapshots_dir: &Path, src: &Path) -> Result<(String, u64), VaultError> {
    let (size, hash) = hash_file(src)?;
    let final_path = object_path(snapshots_dir, &hash);
    if final_path.exists() {
        return Ok((hash, size)); // dedup — the pool already holds this exact ciphertext
    }
    let objs = objects_dir(snapshots_dir);
    std::fs::create_dir_all(&objs).map_err(|e| io_ctx("create objects dir", &e))?;
    let tmp = objs.join(format!("{OBJECT_PREFIX}{hash}.{OBJECT_EXT}.tmp"));
    std::fs::copy(src, &tmp).map_err(|e| io_ctx("copy object to tmp", &e))?;
    match std::fs::rename(&tmp, &final_path) {
        Ok(()) => Ok((hash, size)),
        // A concurrent/previous put already landed the final object — the content is
        // identical by construction, so treat it as success and drop our tmp.
        Err(_) if final_path.exists() => {
            std::fs::remove_file(&tmp).ok();
            Ok((hash, size))
        }
        Err(e) => Err(io_ctx("rename object into pool", &e)),
    }
}

/// A complete snapshot on disk: its directory + parsed manifest.
#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    pub dir: PathBuf,
    pub manifest: SnapshotManifest,
}

impl SnapshotEntry {
    /// The snapshot's directory name (`20260714T153012004Z`), used as its stable id.
    #[must_use]
    pub fn id(&self) -> String {
        self.dir
            .file_name()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
    }

    /// The snapshot's `vault.vdb`.
    #[must_use]
    pub fn vault_file(&self) -> PathBuf {
        self.dir.join(super::manifest::SNAPSHOT_VAULT_FILE)
    }
}

/// Read + parse a snapshot's `manifest.json`.
pub fn read_manifest(dir: &Path) -> Result<SnapshotManifest, VaultError> {
    let path = dir.join(SNAPSHOT_MANIFEST_NAME);
    let bytes = std::fs::read(&path).map_err(|e| io_ctx("read snapshot manifest", &e))?;
    serde_json::from_slice(&bytes).map_err(|e| {
        VaultError::Storage(StorageError::Serialization(format!(
            "snapshot manifest: {e}"
        )))
    })
}

/// Write a snapshot's `manifest.json` durably (tmp → rename). This is a snapshot's commit
/// marker — it is written last, so a dir carrying it is a complete snapshot.
pub fn write_manifest(dir: &Path, manifest: &SnapshotManifest) -> Result<(), VaultError> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| {
        VaultError::Storage(StorageError::Serialization(format!(
            "snapshot manifest: {e}"
        )))
    })?;
    let path = dir.join(SNAPSHOT_MANIFEST_NAME);
    let tmp = dir.join(format!("{SNAPSHOT_MANIFEST_NAME}.tmp"));
    std::fs::write(&tmp, &bytes).map_err(|e| io_ctx("write manifest tmp", &e))?;
    std::fs::rename(&tmp, &path).map_err(|e| io_ctx("rename manifest into place", &e))
}

/// List all COMPLETE snapshots (dirs holding a readable `manifest.json`), **newest first**.
///
/// Plain file reads — NO session, NO KEK, and it **never hashes** an object (a store holding
/// a corrupt object must still list; verification belongs on the restore path). A dir without
/// a readable manifest (an in-flight/crashed create) is skipped, not surfaced. This powers
/// the launch-screen listing of a locked or corrupt vault's snapshots (Decision H0).
pub fn list_snapshots(snapshots_dir: &Path) -> Result<Vec<SnapshotEntry>, VaultError> {
    let mut out = Vec::new();
    if !snapshots_dir.is_dir() {
        return Ok(out);
    }
    let rd = match std::fs::read_dir(snapshots_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(io_ctx("read snapshots dir", &e)),
    };
    for entry in rd {
        let entry = entry.map_err(|e| io_ctx("read snapshots entry", &e))?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if name == OBJECTS_DIR || name.starts_with('.') {
            // The shared pool, or a dot-prefixed `.tmp` staging dir (which may briefly hold
            // a manifest before its atomic rename) — never a committed snapshot.
            continue;
        }
        // A readable manifest is the definition of "complete". Skip anything else.
        if let Ok(manifest) = read_manifest(&dir) {
            out.push(SnapshotEntry { dir, manifest });
        }
    }
    // Dir names are ISO-8601 basic, so a reverse lexicographic sort is newest-first.
    out.sort_by(|a, b| b.dir.file_name().cmp(&a.dir.file_name()));
    Ok(out)
}

/// The set of object hashes referenced by every COMPLETE snapshot — the "mark" of
/// mark-and-sweep (Decision ⑪). Pure; unit-testable without disk.
#[must_use]
pub fn reachable_objects(manifests: &[SnapshotManifest]) -> HashSet<String> {
    manifests
        .iter()
        .flat_map(|m| m.objects.iter().map(|o| o.blake3.clone()))
        .collect()
}

/// Parse the content hash out of a pooled object filename (`b3-<hash>.blob` → `<hash>`).
/// `None` for anything that is not a well-formed object name (a `.tmp`, a stray file).
fn object_hash_from_name(name: &str) -> Option<&str> {
    name.strip_prefix(OBJECT_PREFIX)
        .and_then(|s| s.strip_suffix(".blob"))
}

/// 🔴 Mark-and-sweep the object pool. NEVER a refcount.
///
/// 1. Reap INCOMPLETE snapshot dirs (a dir with no readable `manifest.json` — a crashed
///    create, incl. a `.tmp` staging dir).
/// 2. Mark: union the object hashes of every remaining complete snapshot.
/// 3. Sweep: delete every file in `objects/` that no manifest names (incl. `.tmp` orphans).
///
/// Best-effort per-object (a delete that fails just leaves the orphan for the next run) and
/// idempotent. Returns the number of objects deleted. This is the exact fault model the 4.6a
/// orphan-blob sweep already runs.
pub fn sweep_objects(snapshots_dir: &Path) -> Result<u64, VaultError> {
    if !snapshots_dir.is_dir() {
        return Ok(0);
    }
    reap_incomplete_dirs(snapshots_dir)?;

    let manifests: Vec<SnapshotManifest> = list_snapshots(snapshots_dir)?
        .into_iter()
        .map(|e| e.manifest)
        .collect();
    let reachable = reachable_objects(&manifests);

    let objs = objects_dir(snapshots_dir);
    let rd = match std::fs::read_dir(&objs) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(io_ctx("read objects dir", &e)),
    };
    let mut deleted: u64 = 0;
    for entry in rd {
        let entry = entry.map_err(|e| io_ctx("read object entry", &e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        // Delete anything that is not a reachable, well-formed object (this reaps `.tmp`
        // orphans and any unknown file too). NEVER delete a hash a manifest names.
        let keep = object_hash_from_name(name).is_some_and(|h| reachable.contains(h));
        if !keep && std::fs::remove_file(&path).is_ok() {
            deleted = deleted.saturating_add(1);
        }
    }
    Ok(deleted)
}

/// Reap snapshot dirs with no readable manifest (crashed creates / `.tmp` staging). Never
/// touches `objects/` or a complete snapshot.
fn reap_incomplete_dirs(snapshots_dir: &Path) -> Result<(), VaultError> {
    let rd = std::fs::read_dir(snapshots_dir).map_err(|e| io_ctx("read snapshots dir", &e))?;
    for entry in rd {
        let entry = entry.map_err(|e| io_ctx("read snapshots entry", &e))?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if name == OBJECTS_DIR {
            continue;
        }
        // A dot-prefixed `.tmp` staging dir is always a crashed create (its content never
        // committed via the atomic rename) → reap regardless of a stray manifest. A non-dot
        // dir is complete iff it carries a manifest.
        if !name.starts_with('.') && dir.join(SNAPSHOT_MANIFEST_NAME).is_file() {
            continue; // complete
        }
        // Incomplete → reap (best-effort; a failure just leaves it for the next run).
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(io_ctx("reap incomplete snapshot dir", &e));
            }
        }
    }
    Ok(())
}

/// Resolve + validate a snapshot id (a directory name) against a store, refusing traversal.
///
/// Returns the canonical snapshot directory. The frontend supplies this id, so it is
/// UNTRUSTED — both sides are canonicalized and containment is asserted.
pub fn resolve_snapshot_dir(snapshots_dir: &Path, id: &str) -> Result<PathBuf, VaultError> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(io_msg(format!("unsafe snapshot id: {id}")));
    }
    let candidate = snapshots_dir.join(id);
    let canon_store = snapshots_dir
        .canonicalize()
        .map_err(|e| io_ctx("canonicalize store", &e))?;
    let canon_candidate = candidate
        .canonicalize()
        .map_err(|e| io_ctx("canonicalize snapshot dir", &e))?;
    if !canon_candidate.starts_with(&canon_store) {
        return Err(io_msg(format!("snapshot id escapes the store: {id}")));
    }
    Ok(canon_candidate)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::super::manifest::{SNAPSHOT_FORMAT_VERSION, SnapshotObject, SnapshotReason};
    use super::*;
    use chrono::TimeZone;

    fn manifest_with(objects: Vec<(&str, &str)>) -> SnapshotManifest {
        SnapshotManifest {
            format_version: SNAPSHOT_FORMAT_VERSION,
            reason: SnapshotReason::Manual,
            vault_uuid: Some("01JVAULT".to_owned()),
            schema_version: 1,
            created_at: "2026-07-14T00:00:00.000Z".to_owned(),
            commit_counter: 1,
            verify_hash_prefix: "3f8a1c22b0d4e917".to_owned(),
            entry_count: objects.len() as u64,
            blob_count: objects.len() as u64,
            vault_blake3: "0".repeat(64),
            objects: objects
                .into_iter()
                .map(|(entry_id, hash)| SnapshotObject {
                    entry_id: entry_id.to_owned(),
                    blake3: hash.to_owned(),
                    size: 0,
                })
                .collect(),
        }
    }

    /// Materialize a complete snapshot dir `<snapshots>/<name>/manifest.json`.
    fn write_snapshot_dir(snapshots_dir: &Path, name: &str, manifest: &SnapshotManifest) {
        let dir = snapshots_dir.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vault.vdb"), b"vault bytes").unwrap();
        write_manifest(&dir, manifest).unwrap();
    }

    /// Put a fake object of the given content straight into the pool, returning its hash.
    fn seed_object(snapshots_dir: &Path, content: &[u8]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.blob");
        std::fs::write(&src, content).unwrap();
        put_object(snapshots_dir, &src).unwrap().0
    }

    // Test 1 — naming is UTC, Z-suffixed, colon-free; sorts lexicographically = chronologically.
    #[test]
    fn snapshot_dir_names_sort_chronologically() {
        let mut names = Vec::new();
        for i in 0..10 {
            let ts = chrono::Utc
                .with_ymd_and_hms(2026, 7, 14, 15, 30, i)
                .unwrap();
            names.push(snapshot_dir_name(ts));
        }
        for n in &names {
            assert!(!n.contains(':'), "colon in {n}");
            assert!(n.ends_with('Z'), "no Z suffix in {n}");
        }
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            names, sorted,
            "lexicographic order must equal chronological"
        );
    }

    // Test 2 — CAS dedup: putting the same bytes twice adds ONE object, same hash.
    #[test]
    fn put_object_dedups_identical_content() {
        let store = tempfile::tempdir().unwrap();
        let h1 = seed_object(store.path(), b"same document ciphertext");
        let h2 = seed_object(store.path(), b"same document ciphertext");
        assert_eq!(h1, h2);
        let count = std::fs::read_dir(objects_dir(store.path()))
            .unwrap()
            .filter(|e| e.as_ref().unwrap().path().is_file())
            .count();
        assert_eq!(count, 1, "identical content dedups to one object");
        assert!(object_path(store.path(), &h1).exists());
    }

    // Test 3 — copy-on-change: distinct content → distinct objects; the first is byte-preserved.
    #[test]
    fn put_object_distinct_content_makes_distinct_objects() {
        let store = tempfile::tempdir().unwrap();
        let h1 = seed_object(store.path(), b"document version ONE");
        let h2 = seed_object(store.path(), b"document version TWO");
        assert_ne!(h1, h2);
        assert_eq!(
            std::fs::read(object_path(store.path(), &h1)).unwrap(),
            b"document version ONE",
            "the first object is byte-identical to the pre-change blob"
        );
    }

    // Test 4 — mark-and-sweep: an unshared object of a deleted snapshot is swept; shared
    // objects survive; a manifest-less dir is reaped; idempotent.
    #[test]
    fn sweep_deletes_orphans_keeps_shared_and_is_idempotent() {
        let store = tempfile::tempdir().unwrap();
        let shared = seed_object(store.path(), b"shared across snapshots");
        let only_a = seed_object(store.path(), b"unique to snapshot A");

        // Two complete snapshots; B references only the shared object.
        write_snapshot_dir(
            store.path(),
            "20260714T100000000Z",
            &manifest_with(vec![("E1", &shared), ("E2", &only_a)]),
        );
        write_snapshot_dir(
            store.path(),
            "20260714T110000000Z",
            &manifest_with(vec![("E1", &shared)]),
        );
        // Delete snapshot A → its unique object becomes an orphan.
        std::fs::remove_dir_all(store.path().join("20260714T100000000Z")).unwrap();
        // A manifest-less crashed-create dir + an orphan .tmp object.
        std::fs::create_dir_all(store.path().join(".20260714T120000000Z.tmp")).unwrap();

        let deleted = sweep_objects(store.path()).unwrap();
        assert!(deleted >= 1, "the unshared orphan is swept");
        assert!(
            object_path(store.path(), &shared).exists(),
            "shared survives"
        );
        assert!(!object_path(store.path(), &only_a).exists(), "orphan gone");
        assert!(
            !store.path().join(".20260714T120000000Z.tmp").exists(),
            "manifest-less dir reaped"
        );
        // Idempotent: nothing left to delete.
        assert_eq!(sweep_objects(store.path()).unwrap(), 0);
    }

    // Test 5 — 🔴 the dangerous direction: the sweep NEVER deletes an object referenced only
    // by the OLDEST snapshot.
    #[test]
    fn sweep_never_deletes_an_object_referenced_by_the_oldest_snapshot() {
        let store = tempfile::tempdir().unwrap();
        let old_only = seed_object(store.path(), b"referenced only by the oldest");
        let recent = seed_object(store.path(), b"referenced by the newest");
        write_snapshot_dir(
            store.path(),
            "20260101T000000000Z", // oldest
            &manifest_with(vec![("E1", &old_only)]),
        );
        write_snapshot_dir(
            store.path(),
            "20261231T000000000Z", // newest
            &manifest_with(vec![("E2", &recent)]),
        );
        assert_eq!(sweep_objects(store.path()).unwrap(), 0, "nothing to sweep");
        assert!(
            object_path(store.path(), &old_only).exists(),
            "the oldest snapshot's object must survive"
        );
        assert!(object_path(store.path(), &recent).exists());
    }

    // Test 12 — list_snapshots needs no session and does not hash: a store holding a
    // deliberately corrupt object still lists.
    #[test]
    fn list_snapshots_lists_despite_a_corrupt_object() {
        let store = tempfile::tempdir().unwrap();
        let hash = seed_object(store.path(), b"good bytes");
        write_snapshot_dir(
            store.path(),
            "20260714T100000000Z",
            &manifest_with(vec![("E1", &hash)]),
        );
        // Corrupt the pooled object on disk — its bytes no longer match its name.
        std::fs::write(object_path(store.path(), &hash), b"CORRUPTED").unwrap();
        // An incomplete dir must not appear in the listing.
        std::fs::create_dir_all(store.path().join(".incomplete.tmp")).unwrap();

        let snaps = list_snapshots(store.path()).unwrap();
        assert_eq!(snaps.len(), 1, "the complete snapshot still lists");
        assert_eq!(snaps[0].id(), "20260714T100000000Z");
    }

    #[test]
    fn reachable_objects_unions_all_manifests() {
        let a = manifest_with(vec![("E1", "aa"), ("E2", "bb")]);
        let b = manifest_with(vec![("E3", "bb"), ("E4", "cc")]);
        let reachable = reachable_objects(&[a, b]);
        assert_eq!(reachable.len(), 3);
        assert!(reachable.contains("aa") && reachable.contains("bb") && reachable.contains("cc"));
    }

    #[test]
    fn resolve_snapshot_dir_refuses_traversal() {
        let store = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(store.path().join("20260714T100000000Z")).unwrap();
        assert!(resolve_snapshot_dir(store.path(), "20260714T100000000Z").is_ok());
        assert!(resolve_snapshot_dir(store.path(), "../evil").is_err());
        assert!(resolve_snapshot_dir(store.path(), "").is_err());
        assert!(resolve_snapshot_dir(store.path(), "a/b").is_err());
    }
}
