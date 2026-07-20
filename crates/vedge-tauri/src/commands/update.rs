//! Version + update check (PG.3).
//!
//! 🔴 The check runs entirely **Rust-side**, using the `tauri_plugin_http::reqwest`
//! re-export HIBP already uses (`breach/hibp.rs`) — a library re-export needs no
//! plugin registration. So the frontend keeps **no** HTTP capability and the CSP's
//! `connect-src` stays closed: `capabilities/default.json` and `security.csp` are
//! byte-identical to their PG.2b state (the slice's acceptance criterion, pinned by
//! a test in `commands/update.rs`'s sibling `tests/`).
//!
//! The version is `app.package_info().version` — the value the installer carries,
//! so it stays honest even if `Cargo.toml` ever diverged. Failure is a *status*
//! (`UpdateStatus::Unknown`), never an `Err`: the manual caller shows an inline
//! message; the automatic caller stays silent — neither has to branch on a result.

// Tauri injects `AppHandle` by value into a command (a `&AppHandle` is not an
// injectable arg), and both commands only read it — so `needless_pass_by_value`
// would fire on every command signature. Allowed at the module level.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use semver::Version;
use tauri_plugin_http::reqwest;
use tracing::instrument;

use vedge_ipc::{UpdateCheckDto, UpdateStatus};

/// The **public** GitHub repo whose releases the check reads. Public so the plain
/// GET carries no token — no unique ID, no telemetry (PG.3 ②).
const GITHUB_RELEASES_REPO: &str = "vyngt/vedge";
/// GitHub's API requires a descriptive User-Agent (mirrors `hibp.rs`).
const USER_AGENT: &str = concat!("VEdge/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: Duration = Duration::from_secs(10);

/// The running app version — `app.package_info().version` (the value Tauri
/// bundled). Infallible, so it returns the string directly.
#[must_use]
#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Check the public GitHub releases for a newer stable version.
///
/// Uses `/releases/latest`, which already excludes pre-releases and drafts. A 404
/// (no release published yet) reads as up-to-date, not a failure. Any real failure
/// (network, non-success, unparseable body/tag) resolves to
/// [`UpdateStatus::Unknown`] — see the module docs for why that isn't an `Err`.
#[tauri::command]
#[instrument(skip_all)]
pub async fn check_for_update(app: tauri::AppHandle) -> UpdateCheckDto {
    let current = app.package_info().version.clone();
    let current_str = current.to_string();

    match fetch_latest_release().await {
        Latest::Release { tag, url } => {
            let status = compare(&current, &tag);
            let latest = Some(strip_v(&tag).trim().to_owned());
            // A download link only on a genuine update — never a downgrade CTA.
            let release_url = matches!(status, UpdateStatus::UpdateAvailable).then_some(url);
            UpdateCheckDto {
                current: current_str,
                latest,
                status,
                release_url,
            }
        }
        // 404 — nothing published yet; quietly up-to-date.
        Latest::None => UpdateCheckDto {
            current: current_str,
            latest: None,
            status: UpdateStatus::UpToDate,
            release_url: None,
        },
        Latest::Failed => UpdateCheckDto {
            current: current_str,
            latest: None,
            status: UpdateStatus::Unknown,
            release_url: None,
        },
    }
}

/// The three fetch outcomes (an enum, not `Result<_, ()>`, to avoid the
/// `result_unit_err` lint and to name the 404 case explicitly).
enum Latest {
    Release {
        tag: String,
        url: String,
    },
    /// 404 — no release published yet.
    None,
    /// Network failure, non-success status, or an unreadable body.
    Failed,
}

async fn fetch_latest_release() -> Latest {
    let url = format!("https://api.github.com/repos/{GITHUB_RELEASES_REPO}/releases/latest");
    // Same no-`unwrap`/`expect` fallback as `hibp.rs` (the crate lints deny both).
    let client = reqwest::Client::builder()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let Ok(resp) = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(TIMEOUT)
        .send()
        .await
    else {
        return Latest::Failed;
    };

    if resp.status().as_u16() == 404 {
        return Latest::None;
    }
    if !resp.status().is_success() {
        return Latest::Failed;
    }

    let Ok(body) = resp.text().await else {
        return Latest::Failed;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
        return Latest::Failed;
    };
    let Some(tag) = json.get("tag_name").and_then(serde_json::Value::as_str) else {
        return Latest::Failed;
    };
    let url = json
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Latest::Release {
        tag: tag.to_owned(),
        url,
    }
}

/// Strip a single leading `v`/`V` from a GitHub tag (`v1.2.3` → `1.2.3`).
fn strip_v(tag: &str) -> &str {
    let tag = tag.trim();
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
}

/// Compare the running version against a release tag. 🔴 Never a downgrade: a
/// build **ahead** of the newest tag (a dev build off `develop`) reads as
/// up-to-date. A tag that isn't valid semver is inert (`Unknown`). The comparison
/// is semver, not string (`1.10.0 > 1.9.0`).
fn compare(current: &Version, tag: &str) -> UpdateStatus {
    match Version::parse(strip_v(tag)) {
        Ok(latest) if latest > *current => UpdateStatus::UpdateAvailable,
        Ok(_) => UpdateStatus::UpToDate,
        Err(_) => UpdateStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{UpdateStatus, Version, compare, strip_v};

    fn ver(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn up_to_date_when_equal() {
        assert_eq!(compare(&ver("1.0.0"), "v1.0.0"), UpdateStatus::UpToDate);
    }

    #[test]
    fn update_available_when_newer() {
        assert_eq!(
            compare(&ver("1.0.0"), "v1.1.0"),
            UpdateStatus::UpdateAvailable
        );
    }

    #[test]
    fn ahead_of_latest_reads_up_to_date() {
        // A dev build off `develop`, ahead of the newest tag — must NOT prompt a
        // downgrade.
        assert_eq!(compare(&ver("1.1.0"), "v1.0.0"), UpdateStatus::UpToDate);
    }

    #[test]
    fn strips_v_prefix_and_tolerates_missing_v() {
        assert_eq!(strip_v("v1.2.3"), "1.2.3");
        assert_eq!(strip_v("V1.2.3"), "1.2.3");
        assert_eq!(strip_v("1.2.3"), "1.2.3");
        // A tag without the conventional `v` still compares.
        assert_eq!(
            compare(&ver("1.2.3"), "1.3.0"),
            UpdateStatus::UpdateAvailable
        );
    }

    #[test]
    fn compares_semver_not_lexicographically() {
        // `1.10.0 > 1.9.0` — false as a string compare, true as semver.
        assert_eq!(
            compare(&ver("1.9.0"), "v1.10.0"),
            UpdateStatus::UpdateAvailable
        );
        assert_eq!(compare(&ver("1.10.0"), "v1.9.0"), UpdateStatus::UpToDate);
    }

    #[test]
    fn malformed_tag_is_inert() {
        assert_eq!(
            compare(&ver("1.0.0"), "not-a-version"),
            UpdateStatus::Unknown
        );
        assert_eq!(compare(&ver("1.0.0"), ""), UpdateStatus::Unknown);
        assert_eq!(compare(&ver("1.0.0"), "v"), UpdateStatus::Unknown);
    }
}
