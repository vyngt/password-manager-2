//! Recovery Key DTO layer (slice 5.7).
//!
//! The commands read/construct these directly; there is no core-struct mapping (the recover
//! command parses the typed display strings, and the enroll output is a single show-once
//! string).

pub use vedge_ipc::{RecoveryEnrollOutputDto, UnlockWithRecoveryKeyInputDto};
