//! Export wire DTO (slice 5.3a).
//!
//! Derivatives only — a path, a count, and whether the file is sealed. No
//! payload and no key material crosses this boundary. (The passphrase for an
//! encrypted export travels *in* as a command argument — like the master
//! password on unlock — and never comes back.)

use serde::{Deserialize, Serialize};

/// The result of a successful `export_entries`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReportDto {
    /// Absolute path of the written file.
    pub dest_path: String,
    /// Entries written (logins only for the CSV format).
    pub entry_count: u64,
    /// True for the sealed envelope, false for the plaintext CSV.
    pub encrypted: bool,
}
