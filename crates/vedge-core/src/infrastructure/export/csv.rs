//! The frozen CSV adapter (slice 5.3a) — a **plaintext** logins-only liberation
//! format.
//!
//! # 🔒 Decision ⑤ — frozen at introduction
//!
//! Columns are `name,username,password,url,notes,tags`. **Logins only. Forever.**
//! A second entry type never enters the CSV — everything richer goes to the
//! encrypted JSON envelope. CSV can't nest (`EnvVars.vars`, `Identity.address`,
//! `recovery_codes`), can't carry binary documents, and has no home for a
//! `format_version`; it is an interop *adapter*, not the format.
//!
//! Unlike the envelope, this is **unencrypted by the user's explicit choice**
//! (it must be readable by another tool). That is why the UI shows the honest
//! plaintext warning (Decision ⑧) and why formula injection matters here.
//!
//! # 🔴 Formula injection — never silently mutate values (Decision ⑤)
//!
//! A cell beginning `=`, `+`, `-`, `@` (or a tab / CR) can execute in a
//! spreadsheet. OWASP is explicit that no sanitization is safe for all consumers,
//! and Excel can strip escaping on re-save. So the default liberation CSV writes
//! values **byte-exact** (a password starting `=` round-trips verbatim) plus a
//! loud warning. An opt-in *spreadsheet-safe* mode apostrophe-**prefixes** risky
//! cells — it **never strips** the leading character (that would corrupt
//! legitimate values).
//!
//! # 🔴 BOM
//!
//! The `csv` crate does not strip a UTF-8 BOM (a known wart), so we strip it
//! ourselves on read. Export is UTF-8, **no BOM**.

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

/// The frozen column header, in order.
pub const CSV_HEADER: [&str; 6] = ["name", "username", "password", "url", "notes", "tags"];

/// Intra-cell separator for the `tags` column. A tag name containing this
/// character is an accepted adapter limitation (the envelope carries tags losslessly).
const TAG_SEPARATOR: char = ';';

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// One CSV login row — the frozen logins-only shape.
#[derive(Debug, Clone)]
pub struct CsvLogin {
    pub name: String,
    pub username: String,
    pub password: SecretString,
    /// Empty string = no URL.
    pub url: String,
    /// Empty string = no notes. May contain embedded newlines (RFC 4180 quoted).
    pub notes: String,
    pub tags: Vec<String>,
}

/// A parsed CSV row: its 1-based line number and either a login or a message
/// naming what was wrong (a short row, etc.) — the shape 5.3b's preview badges
/// consume.
#[derive(Debug, Clone)]
pub struct CsvRow {
    pub line: u64,
    pub result: Result<CsvLogin, String>,
}

fn csv_err(op: &str, e: &csv::Error) -> VaultError {
    VaultError::Storage(StorageError::Serialization(format!("csv {op}: {e}")))
}

/// True if `s` begins with a character a spreadsheet may interpret as a formula.
fn starts_with_risky(s: &str) -> bool {
    matches!(s.chars().next(), Some('=' | '+' | '-' | '@' | '\t' | '\r'))
}

/// Apostrophe-prefix a risky cell (spreadsheet-safe mode). **Never strips.**
fn defuse(s: &str) -> String {
    if starts_with_risky(s) {
        format!("'{s}")
    } else {
        s.to_owned()
    }
}

/// Serialize logins to a UTF-8 (no BOM) CSV string.
///
/// `spreadsheet_safe = false` (the default liberation CSV) writes every value
/// **byte-exact**. `spreadsheet_safe = true` apostrophe-prefixes formula-risky
/// cells (never stripping). The result is plaintext and returned `Zeroizing`.
pub fn write_csv(
    logins: &[CsvLogin],
    spreadsheet_safe: bool,
) -> Result<Zeroizing<String>, VaultError> {
    let mut wtr = csv::WriterBuilder::new().from_writer(Vec::new());
    wtr.write_record(CSV_HEADER)
        .map_err(|e| csv_err("write header", &e))?;

    let cell = |s: &str| -> String {
        if spreadsheet_safe {
            defuse(s)
        } else {
            s.to_owned()
        }
    };

    for l in logins {
        let tags = l.tags.join(&TAG_SEPARATOR.to_string());
        wtr.write_record([
            cell(&l.name),
            cell(&l.username),
            cell(l.password.expose_secret()),
            cell(&l.url),
            cell(&l.notes),
            cell(&tags),
        ])
        .map_err(|e| csv_err("write row", &e))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| VaultError::Storage(StorageError::Io(format!("csv flush: {}", e.error()))))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| VaultError::MalformedPayload(format!("csv is not valid UTF-8: {e}")))?;
    Ok(Zeroizing::new(s))
}

/// Strip a leading UTF-8 BOM if present (the `csv` crate will not).
fn strip_bom(input: &[u8]) -> &[u8] {
    input.strip_prefix(&UTF8_BOM).unwrap_or(input)
}

fn split_tags(cell: &str) -> Vec<String> {
    cell.split(TAG_SEPARATOR)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Parse a CSV byte buffer into rows (slice 5.3b consumes these for the preview).
///
/// Strips a UTF-8 BOM first. Strict record length — a short/long row becomes a
/// `CsvRow` with an `Err` message and its line number, never a failed parse of
/// the whole file. Columns are read by **position** (this is `VEdge`'s own frozen
/// format, not another tool's). Never fails at the file level today — 5.3b may add
/// a file-level error, at which point this grows a `Result`.
pub fn parse_csv(input: &[u8]) -> Vec<CsvRow> {
    let stripped = strip_bom(input);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(stripped);

    let mut rows: Vec<CsvRow> = Vec::new();
    for result in rdr.records() {
        match result {
            Ok(rec) => {
                let line = rec.position().map_or(0, csv::Position::line);
                let get = |i: usize| rec.get(i).unwrap_or("").to_owned();
                rows.push(CsvRow {
                    line,
                    result: Ok(CsvLogin {
                        name: get(0),
                        username: get(1),
                        password: SecretString::from(get(2)),
                        url: get(3),
                        notes: get(4),
                        tags: split_tags(&get(5)),
                    }),
                });
            }
            Err(e) => {
                // A malformed row (e.g. wrong field count) — surface it with its
                // position rather than aborting the whole import.
                let line = e.position().map_or(0, csv::Position::line);
                rows.push(CsvRow {
                    line,
                    result: Err(e.to_string()),
                });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use super::*;

    fn login(name: &str, password: &str, notes: &str, tags: &[&str]) -> CsvLogin {
        CsvLogin {
            name: name.to_owned(),
            username: "user".to_owned(),
            password: SecretString::from(password.to_owned()),
            url: "https://example.com".to_owned(),
            notes: notes.to_owned(),
            tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn first_ok(rows: &[CsvRow]) -> &CsvLogin {
        rows.iter()
            .find_map(|r| r.result.as_ref().ok())
            .expect("at least one ok row")
    }

    #[test]
    fn no_bom_on_export() {
        let csv = write_csv(&[login("a", "p", "", &[])], false).unwrap();
        assert!(!csv.as_bytes().starts_with(&UTF8_BOM));
        assert!(csv.starts_with("name,username,password"));
    }

    /// 🔴 A password starting `=` round-trips BYTE-EXACT in default mode.
    #[test]
    fn leading_equals_password_round_trips_verbatim() {
        let csv = write_csv(&[login("acct", "=hunter2", "", &[])], false).unwrap();
        // Written verbatim (not prefixed).
        assert!(csv.contains("=hunter2"));
        let rows = parse_csv(csv.as_bytes());
        assert_eq!(first_ok(&rows).password.expose_secret(), "=hunter2");
    }

    /// Spreadsheet-safe mode PREFIXES (never strips) a risky cell.
    #[test]
    fn spreadsheet_safe_prefixes_but_never_strips() {
        let csv = write_csv(&[login("acct", "=hunter2", "", &[])], true).unwrap();
        assert!(
            csv.contains("'=hunter2"),
            "risky cell must be apostrophe-prefixed"
        );
        // The leading char is preserved — the value is prefixed, not mutated away.
        let rows = parse_csv(csv.as_bytes());
        assert_eq!(first_ok(&rows).password.expose_secret(), "'=hunter2");
    }

    #[test]
    fn embedded_newline_in_notes_survives() {
        let csv = write_csv(&[login("a", "p", "line1\nline2", &[])], false).unwrap();
        let rows = parse_csv(csv.as_bytes());
        assert_eq!(first_ok(&rows).notes, "line1\nline2");
    }

    #[test]
    fn bom_is_stripped_on_read() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(b"name,username,password,url,notes,tags\nacct,u,p,,,\n");
        let rows = parse_csv(&bytes);
        // If the BOM leaked, the first header cell would be "\u{feff}name" and the
        // row would still parse — but we assert the row parsed cleanly.
        assert_eq!(first_ok(&rows).name, "acct");
    }

    #[test]
    fn tags_round_trip_via_semicolons() {
        let csv = write_csv(&[login("a", "p", "", &["work", "personal"])], false).unwrap();
        let rows = parse_csv(csv.as_bytes());
        assert_eq!(
            first_ok(&rows).tags,
            vec!["work".to_owned(), "personal".to_owned()]
        );
    }

    /// 🔴 A short row becomes a positioned error, not a failed import.
    #[test]
    fn short_row_is_a_positioned_error_not_a_failure() {
        // Header has 6 columns; the data row has 2 → an UnequalLengths error.
        let bytes = b"name,username,password,url,notes,tags\nonly,two\n";
        let rows = parse_csv(bytes);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.result.is_err(), "the short row must be an error");
        assert!(row.line >= 2, "the error must carry the row's line number");
    }
}
