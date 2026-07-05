//! Presentation helpers shared by the entry list (`vault_table`) and detail
//! panel (`vault_detail`). `short_date` is pure/host-testable; the entry-type
//! label is localized (`vault.type_*`) so it needs the i18n context.

use crate::i18n::*;
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

/// Trim an RFC-3339 timestamp to its date portion (`2026-07-05T…` → `2026-07-05`).
/// Returns the input unchanged if there's no `T` separator.
pub fn short_date(rfc3339: &str) -> String {
    rfc3339
        .split_once('T')
        .map_or_else(|| rfc3339.to_string(), |(date, _)| date.to_string())
}

#[cfg(test)]
mod tests {
    use super::short_date;

    #[test]
    fn short_date_trims_time() {
        assert_eq!(short_date("2026-07-05T12:00:00.000Z"), "2026-07-05");
    }

    #[test]
    fn short_date_passes_through_without_separator() {
        assert_eq!(short_date("2026-07-05"), "2026-07-05");
    }
}
