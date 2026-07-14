//! Local snapshot store (slice 5.2.1): a content-addressed object pool + versioned
//! snapshot directories inside a vault home's `snapshots/`.
//!
//! This is the **TIME** half of Decision ⑦ — capture this vault's state locally, cheaply and
//! repeatedly, and revert it in place. A snapshot is a *directory*, not a tar (Decision ⑨ —
//! you cannot patch a tar), so a credential change rewraps a few kilobytes rather than
//! re-writing every blob (see [`crate::application::vault::use_cases::rewrap_snapshots`]).
//!
//! [`store`] holds the pure filesystem machinery (the CAS pool + mark-and-sweep GC);
//! [`manifest`] defines the on-disk [`manifest::SnapshotManifest`].

pub mod manifest;
pub mod store;
