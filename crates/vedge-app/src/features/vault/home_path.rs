//! Composing a vault-home path from a **name** + a **location** (slice 5.2.0; shared 5.2.2).
//!
//! A vault is one folder — `<location>/<name>.vedge/` — so any surface that *creates* one has
//! to build that string the same way. There are two such surfaces now: the create-vault wizard
//! (`vault_setup`) and the Open-a-backup dialog (`backup_open_dialog`).
//!
//! 🔴 They must agree exactly, or the same typed input would produce two different paths and
//! the user would silently end up with a vault somewhere they did not choose. Hence: one module,
//! no second copy. These are pure functions, so they are host-testable — no Leptos, no DOM.

/// Ensure a vault name normalizes to a `.vedge` home folder name (slice 5.2.0).
/// A user may type a bare name or a legacy `.vdb`; both become a `.vedge` home.
/// An already-`.vedge` name passes through unchanged.
pub fn ensure_vedge_home(input: &str) -> String {
    let trimmed = input.trim();
    match std::path::Path::new(trimmed)
        .extension()
        .and_then(|s| s.to_str())
    {
        Some("vedge") => trimmed.to_owned(),
        // A legacy `.vdb` typed in the field → swap the extension for `.vedge`.
        Some("vdb") => format!("{}.vedge", trimmed.strip_suffix(".vdb").unwrap_or(trimmed)),
        _ => format!("{trimmed}.vedge"),
    }
}

/// Normalize a typed vault **name** into a `.vedge` home folder name: strip any path separators
/// (a name is a single folder, not a path) then apply [`ensure_vedge_home`]. Empty when blank.
pub fn home_folder_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .filter(|c| *c != '/' && *c != '\\')
        .collect();
    if cleaned.is_empty() {
        String::new()
    } else {
        ensure_vedge_home(&cleaned)
    }
}

/// Infer the path separator from a location string: a backslash for Windows-style paths
/// (containing `\` or an `X:` drive prefix), a forward slash otherwise. Mirrors the
/// platform-native folder picker's output so the composed home matches whatever the OS returned.
pub fn path_sep(location: &str) -> char {
    let mut chars = location.chars();
    let drive = matches!(
        (chars.next(), chars.next()),
        (Some(c), Some(':')) if c.is_ascii_alphabetic()
    );
    if location.contains('\\') || drive {
        '\\'
    } else {
        '/'
    }
}

/// Compose the full vault-home path from a parent `location` and a vault `name`.
/// Returns an empty string when either input is blank.
pub fn compose_home(location: &str, name: &str) -> String {
    let folder = home_folder_name(name);
    let loc = location.trim();
    if loc.is_empty() || folder.is_empty() {
        return String::new();
    }
    let base = loc.trim_end_matches(['/', '\\']);
    let sep = path_sep(loc);
    format!("{base}{sep}{folder}")
}

/// Middle-truncate a path for display: keep the head and the tail — the vault name lives at the
/// end — replacing the middle with `…`, so a long path stays recognizable at both ends. `max` is
/// a **character** budget (including the `…`). Returns the input unchanged when it already fits.
///
/// 🔴 Never the `dir="rtl"` CSS trick (it reorders punctuation and mangles Windows paths). Written
/// in Rust and char-boundary-safe: the workspace denies `indexing_slicing` + `arithmetic_side_effects`,
/// and naive byte slicing panics on a multi-byte boundary. The tail (the vault name) gets the extra
/// character when the budget is odd.
#[must_use]
pub fn middle_truncate(path: &str, max: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max {
        return path.to_owned();
    }
    let budget = max.saturating_sub(1); // one character for the ellipsis
    let head_len = budget.div_euclid(2);
    let tail_len = budget.saturating_sub(head_len); // the vault name gets the extra char
    let head: String = chars.iter().take(head_len).collect();
    let tail: String = chars
        .iter()
        .skip(chars.len().saturating_sub(tail_len))
        .collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    // `.ends_with(".vedge")` asserts the vault name survives at the tail — it is not a real
    // file-extension check (the input is a truncated display string).
    #![allow(clippy::case_sensitive_file_extension_comparisons)]
    use super::*;

    #[test]
    fn a_bare_name_becomes_a_vedge_home() {
        assert_eq!(home_folder_name("work"), "work.vedge");
        assert_eq!(home_folder_name("  work  "), "work.vedge");
    }

    #[test]
    fn a_legacy_vdb_name_is_upgraded_not_double_suffixed() {
        assert_eq!(home_folder_name("work.vdb"), "work.vedge");
        assert_eq!(home_folder_name("work.vedge"), "work.vedge");
    }

    #[test]
    fn a_name_is_one_folder_never_a_path() {
        // A name is a folder, not a path. Separators are stripped, so whatever the user types
        // becomes exactly ONE component under the location they chose — it cannot walk out of it.
        assert_eq!(home_folder_name("a/b\\c"), "abc.vedge");

        // Note what "safe" means here. `../../etc` does not become a traversal; it becomes a
        // folder literally NAMED `....etc.vedge`. Ugly, but harmless — and the property to assert
        // is the one that matters: the name added no separator, so the home is still directly
        // inside the location.
        let home = compose_home("/home/me", "../../etc");
        let tail = home
            .strip_prefix("/home/me/")
            .expect("still under the location");
        assert!(
            !tail.contains('/') && !tail.contains('\\'),
            "a typed name must never introduce a path separator, got {tail:?}"
        );
    }

    #[test]
    fn an_empty_name_or_location_composes_to_nothing() {
        assert_eq!(home_folder_name("   "), "");
        assert_eq!(compose_home("/home/me", ""), "");
        assert_eq!(compose_home("", "work"), "");
    }

    #[test]
    fn the_separator_follows_the_location_style() {
        assert_eq!(compose_home("D:\\Vaults", "work"), "D:\\Vaults\\work.vedge");
        assert_eq!(compose_home("/home/me", "work"), "/home/me/work.vedge");
        // A bare drive root still reads as Windows.
        assert_eq!(compose_home("D:", "work"), "D:\\work.vedge");
    }

    #[test]
    fn a_trailing_separator_on_the_location_is_not_doubled() {
        assert_eq!(
            compose_home("D:\\Vaults\\", "work"),
            "D:\\Vaults\\work.vedge"
        );
        assert_eq!(compose_home("/home/me/", "work"), "/home/me/work.vedge");
    }

    #[test]
    fn middle_truncate_leaves_a_fitting_path_unchanged() {
        assert_eq!(middle_truncate("C:\\a", 20), "C:\\a");
        let exact = "C:\\Users\\vy";
        assert_eq!(middle_truncate(exact, exact.chars().count()), exact); // exact fit
        assert_eq!(middle_truncate("short", 100), "short"); // shorter than max
    }

    #[test]
    fn middle_truncate_keeps_the_head_and_the_vault_name_tail() {
        let p = "C:\\Users\\vy\\Documents\\Vaults\\personal.vedge";
        let out = middle_truncate(p, 24);
        assert!(out.chars().count() <= 24, "within budget: {out:?}");
        assert!(out.contains('…'), "has an ellipsis: {out:?}");
        assert!(out.starts_with("C:\\"), "head preserved: {out:?}");
        assert!(
            out.ends_with(".vedge"),
            "the vault name survives at the tail: {out:?}"
        );
    }

    #[test]
    fn middle_truncate_never_panics_on_a_tiny_budget() {
        // budget < ellipsis + tail: saturating math, no panic, no out-of-bounds.
        assert_eq!(middle_truncate("abcdef", 2), "…f");
        assert_eq!(middle_truncate("abcdef", 1), "…");
        assert_eq!(middle_truncate("abcdef", 0), "…");
    }

    #[test]
    fn middle_truncate_respects_multibyte_char_boundaries() {
        // Vietnamese + CJK — naive byte slicing panics on a boundary here.
        let p = "C:\\Tài liệu\\Kho lưu trữ\\cá nhân日本語.vedge";
        let out = middle_truncate(p, 16);
        assert!(out.chars().count() <= 16, "within budget: {out:?}");
        assert!(out.contains('…'));
        // A `String` is always valid UTF-8 — the point is that we never split a char.
        assert!(out.ends_with(".vedge"), "vault name intact: {out:?}");
    }
}
