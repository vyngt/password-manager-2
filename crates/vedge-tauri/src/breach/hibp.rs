//! `HaveIBeenPwned` *Pwned Passwords* k-anonymity range checker (slice 4.4).
//!
//! Uses `tauri_plugin_http::reqwest` — already compiled, already licence-checked,
//! already passing `cargo deny`. The caller (`scan_health`) hashes the secret and
//! passes only the uppercase 5-hex prefix; this makes one HTTPS GET to
//! `api.pwnedpasswords.com/range/{prefix}` and returns the `(suffix, count)`
//! rows. The suffix match happens back in core. `Add-Padding: true` is sent so
//! the response size does not leak how many suffixes matched.

use std::time::Duration;

use async_trait::async_trait;
use tauri_plugin_http::reqwest;

use vedge_core::application::vault::ports::BreachChecker;
use vedge_core::domain::vault::errors::VaultError;

const HIBP_RANGE: &str = "https://api.pwnedpasswords.com/range/";
/// HIBP's guidance requires a descriptive User-Agent.
const USER_AGENT: &str = concat!("VEdge/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct HibpBreachChecker {
    client: reqwest::Client,
}

impl HibpBreachChecker {
    #[must_use]
    pub fn new() -> Self {
        // `build()` can fail only if the TLS backend won't initialize; fall back
        // to the default client rather than propagate (the plugin already
        // initialized the stack). No `unwrap`/`expect` — keeps the lint gate happy.
        let client = reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl Default for HibpBreachChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a HIBP range body: one `SUFFIX:COUNT` per line, CRLF-tolerant. Padding
/// rows (`count == 0`, produced by `Add-Padding: true`) are discarded; malformed
/// lines are skipped rather than failing the whole lookup. Pure — unit-tested.
fn parse_range_body(body: &str) -> Vec<(String, u32)> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (suffix, count) = line.split_once(':')?;
            let count: u32 = count.trim().parse().ok()?;
            if count == 0 {
                return None; // padding row — not a real hit
            }
            Some((suffix.trim().to_ascii_uppercase(), count))
        })
        .collect()
}

#[async_trait]
impl BreachChecker for HibpBreachChecker {
    async fn range(&self, prefix: &str) -> Result<Vec<(String, u32)>, VaultError> {
        let url = format!("{HIBP_RANGE}{prefix}");
        let resp = self
            .client
            .get(&url)
            .header("Add-Padding", "true")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .timeout(TIMEOUT)
            .send()
            .await
            .map_err(|e| VaultError::BreachLookup(format!("request failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(VaultError::BreachLookup(format!(
                "HIBP returned status {}",
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| VaultError::BreachLookup(format!("read body failed: {e}")))?;
        Ok(parse_range_body(&body))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn hibp_parses_range_response() {
        // CRLF line endings, two real hits, one padding row (count 0), and one
        // malformed line — the parser keeps the hits and drops the rest.
        let body = "0018A45C4D1DEF81644B54AB7F969B88D65:1\r\n\
                    00D4F6E8FA6EECAD2A3AA415EEC418D38EC:2\r\n\
                    FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:0\r\n\
                    garbage-no-colon\r\n";
        let rows = parse_range_body(body);
        assert_eq!(rows.len(), 2, "two real hits; padding + malformed dropped");
        assert!(rows.contains(&("0018A45C4D1DEF81644B54AB7F969B88D65".to_owned(), 1)));
        assert!(rows.contains(&("00D4F6E8FA6EECAD2A3AA415EEC418D38EC".to_owned(), 2)));
        assert!(rows.iter().all(|(s, _)| s.len() == 35), "35-hex suffixes");
        assert!(!rows.iter().any(|(_, c)| *c == 0), "no padding survives");
    }

    #[test]
    fn hibp_uppercases_suffix() {
        let rows = parse_range_body("abcdef0123456789abcdef0123456789abc:7\n");
        assert_eq!(rows[0].0, "ABCDEF0123456789ABCDEF0123456789ABC");
        assert_eq!(rows[0].1, 7);
    }

    #[test]
    fn hibp_empty_body_is_no_matches() {
        assert!(parse_range_body("").is_empty());
        assert!(parse_range_body("\r\n\r\n").is_empty());
    }
}
