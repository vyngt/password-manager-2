//! `VEdge`'s export format (slice 5.3) — the **one permanent artifact** in the product.
//!
//! Its DTO ([`dto`]) binds zero vault schema (Decision ①); the forward [`mapping`]
//! bridges a decrypted `EntryPayload` into it. Restore never routes through here —
//! export is a one-way door for data, not a recovery path.

pub mod dto;
pub mod mapping;

pub use dto::{
    EXPORT_FORMAT_VERSION, ExportAddress, ExportApiKey, ExportBundle, ExportCard, ExportDocument,
    ExportEntry, ExportEnvVar, ExportEnvVars, ExportFolder, ExportIdentity, ExportLogin,
    ExportMeta, ExportNote, ExportSshKey, ExportTotpAlgorithm, ExportTotpParams, ExportUnknown,
};
pub use mapping::payload_to_export;
