//! Audit-log feature — the read path over the trail the core has written since
//! Phase 1. `filters` is pure query-building; `audit_table` is the `DataTable`
//! composition. The page lives in `pages/v/manage/audit.rs`.

pub mod audit_table;
pub mod filters;
