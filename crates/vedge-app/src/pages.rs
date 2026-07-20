pub mod not_found;
pub mod page;
// Dev-only component gallery — compiled out of release builds (PG.2a). Reachable
// only by typing the URL, unaudited, and dead weight in a shipped password
// manager. `mise ci` + `mise e2e` build debug and keep it. (The `playground`
// i18n namespace stays regardless — `vault_search.rs` consumes `playground.clear`.)
#[cfg(debug_assertions)]
pub mod playground;
pub mod v;
