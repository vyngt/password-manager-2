//! In-memory tar container for the export envelope (slice 5.3a).
//!
//! Unlike `backup/archive.rs` — which tars from on-disk paths into a file — this
//! builds and reads the tar entirely in RAM. Decision ②: the plaintext tar must
//! never touch disk, so it exists only as bytes that flow straight into (or out
//! of) the AEAD envelope. The buffers hold plaintext and are returned zeroizing.

use std::io::Read;

use zeroize::Zeroizing;

use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

/// The bundle member: the `ExportBundle` JSON.
pub const ENTRIES_MEMBER: &str = "entries.json";
/// Prefix for document-blob members: `blobs/{source_ulid}` (plaintext bytes).
pub const BLOBS_PREFIX: &str = "blobs/";

fn io_err(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Build an uncompressed tar of `(name, bytes)` members entirely in memory.
///
/// The result holds plaintext (the bundle + document bytes) and is returned
/// `Zeroizing`. Pre-sized so the growing `Vec` does not realloc-and-abandon
/// plaintext buffers un-wiped (the memory-hygiene realloc pitfall).
pub fn build_tar(members: &[(String, &[u8])]) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let est = members
        .iter()
        .map(|(_, d)| d.len().saturating_add(512))
        .sum::<usize>()
        .saturating_add(1024);
    let mut buf: Vec<u8> = Vec::with_capacity(est);
    {
        let mut builder = tar::Builder::new(&mut buf);
        for (name, data) in members {
            let mut header = tar::Header::new_gnu();
            header.set_size(u64::try_from(data.len()).unwrap_or(u64::MAX));
            header.set_mode(0o600);
            // `append_data` sets the path and the checksum itself; we only set
            // size + mode beforehand.
            builder
                .append_data(&mut header, name, *data)
                .map_err(|e| io_err("tar append", &e))?;
        }
        builder.finish().map_err(|e| io_err("tar finish", &e))?;
    }
    Ok(Zeroizing::new(buf))
}

/// Read a single named member from an in-memory tar, or `None` if absent.
/// Used by the round-trip tests (5.3a) and import (5.3b).
pub fn read_member(tar_bytes: &[u8], name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, VaultError> {
    let mut ar = tar::Archive::new(tar_bytes);
    for entry in ar.entries().map_err(|e| io_err("tar entries", &e))? {
        let mut entry = entry.map_err(|e| io_err("tar entry", &e))?;
        let is_target = entry.path().is_ok_and(|p| p.to_string_lossy() == name);
        if is_target {
            let mut out = Vec::new();
            entry
                .read_to_end(&mut out)
                .map_err(|e| io_err("read member", &e))?;
            return Ok(Some(Zeroizing::new(out)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn build_then_read_round_trips_members() {
        let entries = br#"{"format_version":1,"entries":[]}"#;
        let blob = b"\x00\x01\x02 binary doc bytes \xff";
        let members: Vec<(String, &[u8])> = vec![
            (ENTRIES_MEMBER.to_owned(), entries.as_slice()),
            (
                format!("{BLOBS_PREFIX}01ARZ3NDEKTSV4RRFFQ69G5FAV"),
                blob.as_slice(),
            ),
        ];
        let tar = build_tar(&members).unwrap();

        let got_entries = read_member(&tar, ENTRIES_MEMBER).unwrap().unwrap();
        assert_eq!(&*got_entries, entries);

        let got_blob = read_member(&tar, "blobs/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .unwrap()
            .unwrap();
        assert_eq!(&*got_blob, blob);

        assert!(read_member(&tar, "nope").unwrap().is_none());
    }
}
