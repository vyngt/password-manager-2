//! Breach-detection infrastructure (slice 4.4).
//!
//! The real HTTP-backed `HaveIBeenPwned` k-anonymity checker lives here (where
//! `reqwest` already lives via `tauri-plugin-http`), not in `vedge-core` — keeping
//! core offline. The port and its offline test doubles live in `vedge-core`.

pub mod hibp;

pub use hibp::HibpBreachChecker;
