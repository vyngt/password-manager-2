//! The `.vbk` tar container: write, hash, read the manifest, and verify.
//!
//! Uncompressed tar (payload is AEAD ciphertext — incompressible). Every member
//! is streamed (`tar` + a `blake3::Hasher` never hold a whole 50 MB blob
//! resident). Errors map to `VaultError::Storage(StorageError::{Io,Serialization})`.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

use super::manifest::{BackupManifest, MANIFEST_NAME};

fn io_err(op: &str, e: &io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// A writer that tees every byte into a BLAKE3 hasher as it passes through, so
/// the whole-archive digest can never drift from the bytes written to disk.
struct HashingWriter<W: Write> {
    inner: W,
    hasher: blake3::Hasher,
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Write ALL of `buf`, then hash exactly what was written.
        self.inner.write_all(buf)?;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Stream-hash a reader with BLAKE3, returning (bytes, lowercase-hex digest).
fn hash_reader<R: Read>(mut r: R) -> Result<(u64, String), VaultError> {
    let mut hasher = blake3::Hasher::new();
    let bytes = io::copy(&mut r, &mut hasher).map_err(|e| io_err("hash", &e))?;
    Ok((bytes, hasher.finalize().to_hex().to_string()))
}

/// Stream-hash a file with BLAKE3, returning (size, lowercase-hex digest). The
/// backup use case builds `ManifestFile` rows from this — never slurps the blob.
pub fn hash_file(path: &Path) -> Result<(u64, String), VaultError> {
    let f = File::open(path).map_err(|e| io_err("open for hash", &e))?;
    hash_reader(BufReader::new(f))
}

/// Write an uncompressed tar of `members` to `dest`, returning the whole-archive
/// BLAKE3 (lowercase hex).
///
/// Each member is `(tar_name, source_path)`; names are used verbatim
/// (`manifest.json`, `vault.vdb`, `blobs/{ulid}.blob`) and order is preserved.
pub fn write_archive(members: &[(String, PathBuf)], dest: &Path) -> Result<String, VaultError> {
    let file = File::create(dest).map_err(|e| io_err("create archive", &e))?;
    let mut tee = HashingWriter {
        inner: BufWriter::new(file),
        hasher: blake3::Hasher::new(),
    };
    {
        let mut builder = tar::Builder::new(&mut tee);
        for (name, src) in members {
            builder
                .append_path_with_name(src, name)
                .map_err(|e| io_err("tar append", &e))?;
        }
        builder.finish().map_err(|e| io_err("tar finish", &e))?;
    }
    tee.flush().map_err(|e| io_err("flush archive", &e))?;
    Ok(tee.hasher.finalize().to_hex().to_string())
}

/// Read (only) the `manifest.json` member from an archive.
pub fn read_manifest(archive: &Path) -> Result<BackupManifest, VaultError> {
    let f = File::open(archive).map_err(|e| io_err("open archive", &e))?;
    let mut ar = tar::Archive::new(BufReader::new(f));
    let entries = ar.entries().map_err(|e| io_err("tar entries", &e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| io_err("tar entry", &e))?;
        let is_manifest = entry
            .path()
            .is_ok_and(|p| p.to_string_lossy() == MANIFEST_NAME);
        if is_manifest {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| io_err("read manifest", &e))?;
            return serde_json::from_str(&buf).map_err(|e| {
                VaultError::Storage(StorageError::Serialization(format!("manifest: {e}")))
            });
        }
    }
    Err(VaultError::Storage(StorageError::Io(
        "archive is missing manifest.json".to_owned(),
    )))
}

/// The advisory whole-archive hash sidecar written beside a `.vbk`: `<archive>.blake3`.
///
/// ONE source of truth for the path — `backup_vault` writes it and [`verify_archive`]
/// reads it, and a drift between the two would silently disable the check.
#[must_use]
pub fn sidecar_path(archive: &Path) -> PathBuf {
    let mut s = archive.as_os_str().to_owned();
    s.push(".blake3");
    PathBuf::from(s)
}

/// Verify the advisory `.blake3` sidecar **when it is present** (finding L2).
///
/// Why it exists: the manifest lives INSIDE the tar and nothing else authenticates it.
/// `verify_archive` hashes each member against `manifest.files` — which is
/// self-referential, so an archive whose manifest *and* members were both rewritten
/// passes that gate. The sidecar is the only check that spans the whole file.
///
/// 5.2's Decision ③ promised "advisory: verified when present", and 5.2 then shipped a
/// writer with no reader. This is the reader. **Advisory** means exactly this:
/// - absent (or unreadable, or empty) → **pass**. A backup made before this shipped, or
///   copied without its sidecar, must still restore.
/// - present and non-empty → it **must** match, or the archive is refused.
///
/// Cost: one extra full-file read, and ONLY when a sidecar exists.
fn verify_sidecar(archive: &Path) -> Result<(), VaultError> {
    let Ok(recorded) = std::fs::read_to_string(sidecar_path(archive)) else {
        return Ok(()); // no sidecar → advisory → nothing to check
    };
    let recorded = recorded.trim();
    if recorded.is_empty() {
        // A torn write (`backup_vault` warns and carries on). Nothing to compare against.
        return Ok(());
    }
    let (_bytes, actual) = hash_file(archive)?;
    if !recorded.eq_ignore_ascii_case(&actual) {
        return Err(VaultError::Storage(StorageError::Io(
            "backup integrity check failed: the archive does not match its .blake3 sidecar"
                .to_owned(),
        )));
    }
    Ok(())
}

/// Verify every non-manifest member against the manifest's per-file BLAKE3, and the
/// whole archive against its `.blake3` sidecar when one is present.
///
/// Streams each member through the hasher. Returns the manifest on success; on
/// the first mismatch / missing member, returns an error **naming it**. The
/// mandatory integrity gate — restore calls this BEFORE touching any live file.
pub fn verify_archive(archive: &Path) -> Result<BackupManifest, VaultError> {
    // The outermost gate first: the sidecar is the only thing that authenticates the
    // manifest itself, so a mismatch here means nothing below can be trusted (L2).
    verify_sidecar(archive)?;

    let manifest = read_manifest(archive)?;

    let f = File::open(archive).map_err(|e| io_err("open archive", &e))?;
    let mut ar = tar::Archive::new(BufReader::new(f));
    let entries = ar.entries().map_err(|e| io_err("tar entries", &e))?;

    let mut seen: HashSet<String> = HashSet::new();
    for entry in entries {
        let mut entry = entry.map_err(|e| io_err("tar entry", &e))?;
        let name = entry
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name == MANIFEST_NAME {
            continue;
        }
        let expected = manifest
            .files
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| {
                VaultError::Storage(StorageError::Io(format!(
                    "archive member not in manifest: {name}"
                )))
            })?;
        if !seen.insert(name.clone()) {
            return Err(VaultError::Storage(StorageError::Io(format!(
                "archive has a duplicate member: {name}"
            ))));
        }
        let (_bytes, digest) = hash_reader(&mut entry)?;
        if digest != expected.blake3 {
            return Err(VaultError::Storage(StorageError::Io(format!(
                "backup integrity check failed for {name}"
            ))));
        }
    }
    // Every manifest-listed member must have been present and verified. A bare
    // count would be satisfied by a duplicate member standing in for a missing
    // one (dup `A.blob` + absent `B.blob` → same count, wrong contents).
    for m in &manifest.files {
        if !seen.contains(&m.name) {
            return Err(VaultError::Storage(StorageError::Io(format!(
                "archive is missing manifest member: {}",
                m.name
            ))));
        }
    }
    Ok(manifest)
}

/// Extract a single named member from an archive to `dest` (streaming). Restore
/// (5.2b) extracts the whole archive; this single-member form backs snapshot
/// inspection and tests.
pub fn extract_member(archive: &Path, name: &str, dest: &Path) -> Result<(), VaultError> {
    let f = File::open(archive).map_err(|e| io_err("open archive", &e))?;
    let mut ar = tar::Archive::new(BufReader::new(f));
    let entries = ar.entries().map_err(|e| io_err("tar entries", &e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| io_err("tar entry", &e))?;
        let is_target = entry.path().is_ok_and(|p| p.to_string_lossy() == name);
        if is_target {
            let mut out = File::create(dest).map_err(|e| io_err("create extracted member", &e))?;
            io::copy(&mut entry, &mut out).map_err(|e| io_err("extract member", &e))?;
            return Ok(());
        }
    }
    Err(VaultError::Storage(StorageError::Io(format!(
        "archive member not found: {name}"
    ))))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::super::manifest::{BACKUP_FORMAT_VERSION, ManifestFile};
    use super::*;

    fn manifest_for(files: Vec<ManifestFile>) -> BackupManifest {
        BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            vault_uuid: Some("01J0TESTUUID".to_owned()),
            schema_version: 1,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            entry_count: 0,
            blob_count: 0,
            commit_counter: 0,
            verify_hash_prefix: None,
            files,
        }
    }

    #[test]
    fn write_read_verify_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.vdb");
        std::fs::write(&vault, b"hello vault bytes").unwrap();
        let (size, digest) = hash_file(&vault).unwrap();

        let manifest = manifest_for(vec![ManifestFile {
            name: "vault.vdb".to_owned(),
            size,
            blake3: digest,
        }]);
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let dest = dir.path().join("out.vbk");
        let members = vec![
            ("manifest.json".to_owned(), manifest_path),
            ("vault.vdb".to_owned(), vault),
        ];
        let archive_hash = write_archive(&members, &dest).unwrap();
        assert_eq!(archive_hash.len(), 64, "BLAKE3-256 is 32 bytes = 64 hex");

        assert_eq!(read_manifest(&dest).unwrap(), manifest);
        assert_eq!(verify_archive(&dest).unwrap(), manifest);
    }

    /// Build a small, internally-consistent archive holding `body` as `vault.vdb`,
    /// returning its path and its whole-archive digest.
    fn archive_holding(dir: &Path, body: &[u8]) -> (PathBuf, String) {
        let vault = dir.join("vault.vdb");
        std::fs::write(&vault, body).unwrap();
        let (size, digest) = hash_file(&vault).unwrap();
        let manifest = manifest_for(vec![ManifestFile {
            name: "vault.vdb".to_owned(),
            size,
            blake3: digest,
        }]);
        let manifest_path = dir.join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let dest = dir.join("out.vbk");
        let hash = write_archive(
            &[
                ("manifest.json".to_owned(), manifest_path),
                ("vault.vdb".to_owned(), vault),
            ],
            &dest,
        )
        .unwrap();
        (dest, hash)
    }

    /// L2: a MISSING sidecar is advisory — the archive still verifies. Backups taken
    /// before 5.2.2, or copied without their sidecar, must keep working forever.
    #[test]
    fn verify_passes_without_a_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let (archive, _) = archive_holding(dir.path(), b"contents");
        assert!(!sidecar_path(&archive).exists());
        assert!(verify_archive(&archive).is_ok());
    }

    /// L2: a MATCHING sidecar verifies.
    #[test]
    fn verify_accepts_a_matching_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let (archive, hash) = archive_holding(dir.path(), b"contents");
        // Written exactly as `backup_vault` writes it — with the trailing newline.
        std::fs::write(sidecar_path(&archive), format!("{hash}\n")).unwrap();
        assert!(verify_archive(&archive).is_ok());
    }

    /// 🔴 L2, the case that justifies the whole check: an attacker rewrites the manifest
    /// AND the members together, so the per-member gate (which hashes against the very
    /// manifest that was rewritten) passes happily. The sidecar is the ONLY thing that
    /// spans the whole file, and it catches this.
    #[test]
    fn verify_rejects_an_archive_that_no_longer_matches_its_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let (_original, original_hash) = archive_holding(dir.path(), b"authentic contents");

        // A wholly re-packed archive: different body, and a manifest re-hashed to match it.
        // Self-consistent — the per-member check cannot tell the difference.
        let evil_dir = tempfile::tempdir().unwrap();
        let (evil, evil_hash) = archive_holding(evil_dir.path(), b"substituted contents!");
        assert_ne!(original_hash, evil_hash);
        assert!(
            verify_archive(&evil).is_ok(),
            "the per-member gate is self-referential and passes — this is the hole"
        );

        // Now put the ORIGINAL's sidecar beside it, as if the file had been swapped
        // underneath a sidecar the user still trusts.
        std::fs::write(sidecar_path(&evil), format!("{original_hash}\n")).unwrap();
        let err = verify_archive(&evil).unwrap_err();
        assert!(
            format!("{err}").contains("storage"),
            "expected a storage refusal, got {err:?}"
        );
    }

    #[test]
    fn verify_rejects_tampered_member() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.vdb");
        std::fs::write(&vault, b"authentic contents").unwrap();

        // Manifest claims a WRONG digest for vault.vdb.
        let manifest = manifest_for(vec![ManifestFile {
            name: "vault.vdb".to_owned(),
            size: 18,
            blake3: "0".repeat(64),
        }]);
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let dest = dir.path().join("bad.vbk");
        let members = vec![
            ("manifest.json".to_owned(), manifest_path),
            ("vault.vdb".to_owned(), vault),
        ];
        write_archive(&members, &dest).unwrap();

        let err = verify_archive(&dest).unwrap_err();
        let msg = err.to_string();
        // The error surfaces through the Storage wrapper; the member name is in
        // the source. Assert the integrity gate rejected it.
        assert!(
            format!("{err:?}").contains("vault.vdb") || msg.contains("storage"),
            "expected an integrity failure naming the member, got {err:?}"
        );
    }

    #[test]
    fn verify_rejects_a_duplicate_standing_in_for_a_missing_member() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.vdb");
        std::fs::write(&vault, b"vault bytes").unwrap();
        let blob_a = dir.path().join("A.blob");
        std::fs::write(&blob_a, b"blob A bytes").unwrap();

        let (vsize, vhash) = hash_file(&vault).unwrap();
        let (asize, ahash) = hash_file(&blob_a).unwrap();

        // Manifest lists vault + A + B, but the archive carries A twice and no B —
        // a bare member-count check would be fooled (3 members == 3 files).
        let manifest = manifest_for(vec![
            ManifestFile {
                name: "vault.vdb".to_owned(),
                size: vsize,
                blake3: vhash,
            },
            ManifestFile {
                name: "blobs/A.blob".to_owned(),
                size: asize,
                blake3: ahash.clone(),
            },
            ManifestFile {
                name: "blobs/B.blob".to_owned(),
                size: asize,
                blake3: ahash,
            },
        ]);
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let dest = dir.path().join("dup.vbk");
        let members = vec![
            ("manifest.json".to_owned(), manifest_path),
            ("vault.vdb".to_owned(), vault),
            ("blobs/A.blob".to_owned(), blob_a.clone()),
            ("blobs/A.blob".to_owned(), blob_a),
        ];
        write_archive(&members, &dest).unwrap();

        // Must be rejected — the duplicate can't cover for the missing B.
        assert!(verify_archive(&dest).is_err());
    }
}
