//! Pure, host-testable presentation helpers shared by the entry list
//! (`vault_table`) and detail panel (`vault_detail`). No reactivity here —
//! just `IndexEntryDto` → display-string mapping.

use vedge_ipc::EntryTypeDto;

/// Short English label for an entry type, for the list/detail Type field.
/// (Type names aren't localized yet — a later i18n pass can map these.)
pub fn type_label(entry_type: &EntryTypeDto) -> &'static str {
    match entry_type {
        EntryTypeDto::Login => "Login",
        EntryTypeDto::Card => "Card",
        EntryTypeDto::SshKey => "SSH Key",
        EntryTypeDto::ApiKey => "API Key",
        EntryTypeDto::EnvVars => "Env Vars",
        EntryTypeDto::Note => "Note",
        EntryTypeDto::Document => "Document",
        EntryTypeDto::Identity => "Identity",
        EntryTypeDto::Folder => "Folder",
        EntryTypeDto::Unknown(_) => "Other",
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
    use super::{short_date, type_label};
    use vedge_ipc::EntryTypeDto;

    #[test]
    fn type_label_maps_known_and_unknown() {
        assert_eq!(type_label(&EntryTypeDto::Login), "Login");
        assert_eq!(type_label(&EntryTypeDto::SshKey), "SSH Key");
        assert_eq!(
            type_label(&EntryTypeDto::Unknown("Passkey".into())),
            "Other"
        );
    }

    #[test]
    fn short_date_trims_time() {
        assert_eq!(short_date("2026-07-05T12:00:00.000Z"), "2026-07-05");
    }

    #[test]
    fn short_date_passes_through_without_separator() {
        assert_eq!(short_date("2026-07-05"), "2026-07-05");
    }
}
