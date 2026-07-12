//! Breach-corpus lookup doubles (slice 4.4).
//!
//! The real HTTP-backed checker (`HibpBreachChecker`) lives in `vedge-tauri`,
//! where `reqwest` already lives — putting a TLS stack in `vedge-core` would make
//! core tests network-capable. These are the offline doubles used by unit /
//! integration tests and by the e2e harness's env-gated seam.

pub mod memory;
pub mod noop;

pub use memory::MemoryBreachChecker;
pub use noop::NoopBreachChecker;
