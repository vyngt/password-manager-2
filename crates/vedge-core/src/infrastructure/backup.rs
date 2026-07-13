//! Backup archive (slice 5.2): the `.vbk` container + its manifest.
//!
//! A backup is `tar(manifest.json + vault.vdb + blobs/*.blob)` — **uncompressed**
//! (the payload is AEAD ciphertext, incompressible) with per-file + whole-archive
//! BLAKE3. The `.vdb` snapshot is produced by a live `VACUUM INTO` (see the
//! repository); this module only writes/reads/verifies the container. Restore's
//! intent journal (slice 5.2b) lives in [`journal`].

pub mod archive;
pub mod journal;
pub mod manifest;
pub mod target;
