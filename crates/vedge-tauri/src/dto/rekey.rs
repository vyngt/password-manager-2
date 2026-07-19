//! Re-key DTO layer (slice 5.8).
//!
//! The command reads/constructs these directly — the credentials are wrapped in `Zeroizing`
//! inside the command, and the result carries only the show-once new Secret-Key display.

pub use vedge_ipc::{RekeyInputDto, RekeyProgressDto, RekeyResultDto};
