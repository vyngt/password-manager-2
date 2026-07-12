//! Password-health feature (slice 4.3b) — the read surface over the audit-silent
//! bulk scan the core exposes as `scan_health`. `context` holds the ephemeral
//! report (wiped on lock); `summary` and `health_table` compose the `/v/health`
//! page's header + findings table. The page itself lives in
//! `pages/v/manage/health.rs`.

pub mod context;
pub mod health_table;
pub mod summary;
