//! Read-side vault queries.
//!
//! Every shell read command routes through one of these. Each function
//! is a thin `async fn` over `VaultIndex` — no I/O, no mutation. Owned
//! returns (`Vec<IndexEntry>` / `Vec<TagMeta>`) keep the signatures free
//! of lifetime parameters threaded from the session's mutex guard.
//!
//! Takes `&VaultIndex` rather than `&VaultSession` so the dependency is
//! honest (these queries don't need crypto/repo/blob/clipboard) and so
//! tests can construct an index directly without stubbing five ports.

use tracing::instrument;

use crate::domain::shared::{EntryId, TagId};
use crate::domain::vault::index::{IndexEntry, TagMeta, VaultIndex};

/// Every non-trashed entry.
#[instrument(skip_all)]
pub async fn list_active_entries(index: &VaultIndex) -> Vec<IndexEntry> {
    index.all_active().into_iter().cloned().collect()
}

/// Every trashed entry (pre-purge retention window).
#[instrument(skip_all)]
pub async fn list_trashed_entries(index: &VaultIndex) -> Vec<IndexEntry> {
    index.all_trashed().into_iter().cloned().collect()
}

/// Case-insensitive substring search over name + URL. Empty / whitespace
/// query returns all active entries.
#[instrument(skip_all, fields(query_len = query.len()))]
pub async fn search_entries(index: &VaultIndex, query: &str) -> Vec<IndexEntry> {
    index.search(query).into_iter().cloned().collect()
}

/// Every entry carrying the given tag.
#[instrument(skip_all, fields(tag_id = %tag_id))]
pub async fn entries_by_tag(index: &VaultIndex, tag_id: &TagId) -> Vec<IndexEntry> {
    index.by_tag(tag_id).into_iter().cloned().collect()
}

/// Every entry in a folder. `None` = root (entries without a folder).
#[instrument(skip_all)]
pub async fn entries_by_folder(index: &VaultIndex, folder_id: Option<&EntryId>) -> Vec<IndexEntry> {
    index.by_folder(folder_id).into_iter().cloned().collect()
}

/// Every entry whose URL matches the given domain (case-insensitive).
#[instrument(skip_all, fields(domain = %domain))]
pub async fn entries_by_domain(index: &VaultIndex, domain: &str) -> Vec<IndexEntry> {
    index.by_domain(domain).into_iter().cloned().collect()
}

/// Every tag, sorted by `(sort_order ASC, name ASC)`.
#[instrument(skip_all)]
pub async fn list_tags(index: &VaultIndex) -> Vec<TagMeta> {
    let mut tags: Vec<TagMeta> = index.tags.values().cloned().collect();
    tags.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
    tags
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
    use crate::domain::shared::now;
    use crate::domain::vault::entities::EntryRow;
    use crate::domain::vault::payloads::{
        CommonMeta, EntryPayload, EntryType, LoginPayload, NotePayload,
    };
    use secrecy::SecretString;

    fn mk_login(name: &str, url: Option<&str>, tag: Option<TagId>) -> EntryPayload {
        let mut meta = CommonMeta::new(name, EntryType::Login);
        meta.url = url.map(ToOwned::to_owned);
        if let Some(t) = tag {
            meta.tag_ids = vec![t];
        }
        EntryPayload::Login(LoginPayload {
            meta,
            username: "u".into(),
            password: SecretString::from("p"),
            totp_secret: None,
            recovery_codes: vec![],
        })
    }

    fn mk_note_in_folder(name: &str, folder: EntryId) -> EntryPayload {
        let mut meta = CommonMeta::new(name, EntryType::Note);
        meta.folder_id = Some(folder);
        EntryPayload::Note(NotePayload {
            meta,
            content: SecretString::from("c"),
        })
    }

    fn mk_row(id: EntryId, is_trashed: bool) -> EntryRow {
        EntryRow {
            id,
            version: 1,
            cipher_suite: 1,
            dek_wrapped: [0u8; 40],
            nonce: [0u8; 24],
            ciphertext: vec![],
            created_at: now(),
            updated_at: now(),
            accessed_at: None,
            is_trashed,
            trashed_at: None,
        }
    }

    fn seeded_index() -> (VaultIndex, EntryId, EntryId, EntryId, TagId) {
        let mut idx = VaultIndex::new();

        // One tag.
        let tag = TagId::new();
        idx.insert_tag(TagMeta {
            id: tag.clone(),
            name: "work".into(),
            color: None,
            sort_order: 0,
        });

        // Active tagged login at example.com.
        let e_active = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &mk_login(
                "GitHub",
                Some("https://example.com/login"),
                Some(tag.clone()),
            ),
            &mk_row(e_active.clone(), false),
        ));

        // Trashed untagged.
        let e_trash = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &mk_login("Old Account", None, None),
            &mk_row(e_trash.clone(), true),
        ));

        // Note in a folder.
        let folder = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &mk_note_in_folder("Meeting notes", folder.clone()),
            &mk_row(EntryId::new(), false),
        ));

        (idx, e_active, e_trash, folder, tag)
    }

    #[tokio::test]
    async fn list_active_excludes_trashed() {
        let (idx, _, _, _, _) = seeded_index();
        let rows = list_active_entries(&idx).await;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|e| !e.is_trashed));
    }

    #[tokio::test]
    async fn list_trashed_includes_only_trashed() {
        let (idx, _, _, _, _) = seeded_index();
        let rows = list_trashed_entries(&idx).await;
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_trashed);
    }

    #[tokio::test]
    async fn search_matches_name_case_insensitive() {
        let (idx, _, _, _, _) = seeded_index();
        let rows = search_entries(&idx, "github").await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "GitHub");
    }

    #[tokio::test]
    async fn by_tag_filters_to_tagged() {
        let (idx, _, _, _, tag) = seeded_index();
        let rows = entries_by_tag(&idx, &tag).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "GitHub");
    }

    #[tokio::test]
    async fn by_folder_filters_to_folder() {
        let (idx, _, _, folder, _) = seeded_index();
        let rows = entries_by_folder(&idx, Some(&folder)).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Meeting notes");
    }

    #[tokio::test]
    async fn by_folder_none_returns_active_root_entries() {
        let (idx, _, _, _, _) = seeded_index();
        let rows = entries_by_folder(&idx, None).await;
        // GitHub is active+rootless; trashed entry is excluded; note has a folder.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "GitHub");
    }

    #[tokio::test]
    async fn by_domain_matches_example_dot_com() {
        let (idx, _, _, _, _) = seeded_index();
        let rows = entries_by_domain(&idx, "example.com").await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "GitHub");
    }

    #[tokio::test]
    async fn list_tags_sorted_by_order_then_name() {
        let mut idx = VaultIndex::new();
        idx.insert_tag(TagMeta {
            id: TagId::new(),
            name: "zeta".into(),
            color: None,
            sort_order: 0,
        });
        idx.insert_tag(TagMeta {
            id: TagId::new(),
            name: "alpha".into(),
            color: None,
            sort_order: 0,
        });
        idx.insert_tag(TagMeta {
            id: TagId::new(),
            name: "last".into(),
            color: None,
            sort_order: 5,
        });
        let tags = list_tags(&idx).await;
        let names: Vec<_> = tags.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["alpha", "zeta", "last"]);
    }
}
