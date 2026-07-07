//! Presentation helpers shared by the entry list (`vault_table`) and detail
//! panel (`vault_detail`). `short_date` is pure/host-testable; the entry-type
//! label is localized (`vault.type_*`) so it needs the i18n context.

use crate::i18n::*;
use icondata as i;
use leptos_i18n::I18nContext;
use vedge_ipc::EntryTypeDto;

/// Localized label for an entry type, for the list/detail Type field.
pub fn type_label_i18n(i18n: I18nContext<Locale>, entry_type: &EntryTypeDto) -> String {
    match entry_type {
        EntryTypeDto::Login => t_string!(i18n, vault.type_login).to_string(),
        EntryTypeDto::Card => t_string!(i18n, vault.type_card).to_string(),
        EntryTypeDto::SshKey => t_string!(i18n, vault.type_ssh_key).to_string(),
        EntryTypeDto::ApiKey => t_string!(i18n, vault.type_api_key).to_string(),
        EntryTypeDto::EnvVars => t_string!(i18n, vault.type_env_vars).to_string(),
        EntryTypeDto::Note => t_string!(i18n, vault.type_note).to_string(),
        EntryTypeDto::Document => t_string!(i18n, vault.type_document).to_string(),
        EntryTypeDto::Identity => t_string!(i18n, vault.type_identity).to_string(),
        EntryTypeDto::Folder => t_string!(i18n, vault.type_folder).to_string(),
        EntryTypeDto::Unknown(_) => t_string!(i18n, vault.type_other).to_string(),
    }
}

/// Icon token for an entry type, for the list / command-palette rows.
#[must_use]
pub fn type_icon(entry_type: &EntryTypeDto) -> icondata::Icon {
    match entry_type {
        EntryTypeDto::Login => i::FaGlobeSolid,
        EntryTypeDto::Card => i::FaCreditCardSolid,
        EntryTypeDto::SshKey => i::FaTerminalSolid,
        EntryTypeDto::ApiKey => i::FaKeySolid,
        EntryTypeDto::EnvVars => i::FaCodeSolid,
        EntryTypeDto::Note => i::FaFileLinesSolid,
        EntryTypeDto::Document => i::FaFileSolid,
        EntryTypeDto::Identity => i::FaIdCardSolid,
        EntryTypeDto::Folder => i::FaFolderSolid,
        EntryTypeDto::Unknown(_) => i::FaCircleQuestionSolid,
    }
}

/// Trim an RFC-3339 timestamp to its date portion (`2026-07-05T…` → `2026-07-05`).
/// Returns the input unchanged if there's no `T` separator.
pub fn short_date(rfc3339: &str) -> String {
    rfc3339
        .split_once('T')
        .map_or_else(|| rfc3339.to_string(), |(date, _)| date.to_string())
}

/// Human-readable byte size for the Document detail view (e.g. `2.5 MB`).
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::{human_size, short_date};

    #[test]
    fn short_date_trims_time() {
        assert_eq!(short_date("2026-07-05T12:00:00.000Z"), "2026-07-05");
    }

    #[test]
    fn short_date_passes_through_without_separator() {
        assert_eq!(short_date("2026-07-05"), "2026-07-05");
    }

    #[test]
    fn human_size_scales_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }
}
