//! In-memory projection of a vault's non-secret metadata.
//!
//! `VaultIndex` is built once at unlock time from decrypted payloads and lives
//! for the session. It is authoritative for every read — use cases never hit
//! `SQLite` for metadata. Every DB write is atomically mirrored into the index
//! so the session stays in sync.
//!
//! The fields tracked here are explicitly **non-secret**: names, URLs, tag
//! IDs, timestamps. Passwords, keys, document contents never enter the index.

use std::collections::HashMap;

use indexmap::IndexMap;
use zeroize::Zeroize;

use crate::domain::shared::{EntryId, TagId, Timestamp};
use crate::domain::vault::entities::EntryRow;
use crate::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType};

/// Lightweight, non-secret projection of an entry. Safe to hold for the
/// entire session.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub id: EntryId,
    pub name: String,
    pub entry_type: EntryType,
    pub url: Option<String>,
    pub favicon_url: Option<String>,
    pub tag_ids: Vec<TagId>,
    pub folder_id: Option<EntryId>,
    pub is_favorite: bool,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: u32,
    pub is_trashed: bool,
    pub cipher_suite: i32,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub accessed_at: Option<Timestamp>,
}

impl IndexEntry {
    /// Build an index entry from the decrypted payload + its raw DB row.
    /// The payload supplies the `CommonMeta` fields; the row supplies
    /// timestamps and cipher suite.
    #[must_use]
    pub fn from_payload(payload: &EntryPayload, row: &EntryRow) -> Self {
        let meta: &CommonMeta = payload.meta();
        Self {
            id: row.id.clone(),
            name: meta.name.clone(),
            entry_type: meta.entry_type.clone(),
            url: meta.url.clone(),
            favicon_url: meta.favicon_url.clone(),
            tag_ids: meta.tag_ids.clone(),
            folder_id: meta.folder_id.clone(),
            is_favorite: meta.is_favorite,
            color: meta.color.clone(),
            icon: meta.icon.clone(),
            sort_order: meta.sort_order,
            is_trashed: row.is_trashed,
            cipher_suite: row.cipher_suite,
            created_at: row.created_at,
            updated_at: row.updated_at,
            accessed_at: row.accessed_at,
        }
    }
}

/// Decrypted tag metadata — the KEK-encrypted `TagPayload` lifted into
/// in-memory form. The `name` field is stored in normalized form (lowercase,
/// trimmed, internal whitespace collapsed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMeta {
    pub id: TagId,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: u32,
}

/// In-memory index of a decrypted vault.
///
/// - `entries` is an `IndexMap` so iteration order matches insertion order,
///   which we seed as `updated_at` descending at unlock time.
/// - The three derived indexes (`tag_index`, `url_index`, `folders`) are
///   rebuilt from `entries` + `tags` on every mutation — always consistent
///   with the canonical `entries` map.
pub struct VaultIndex {
    pub entries: IndexMap<EntryId, IndexEntry>,
    pub tags: HashMap<TagId, TagMeta>,
    pub tag_index: HashMap<TagId, Vec<EntryId>>,
    pub url_index: HashMap<String, Vec<EntryId>>,
    pub folders: HashMap<EntryId, Vec<EntryId>>,
}

impl VaultIndex {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
            tags: HashMap::new(),
            tag_index: HashMap::new(),
            url_index: HashMap::new(),
            folders: HashMap::new(),
        }
    }

    // ---- Read queries (UI surface) -----------------------------------------

    /// Full-text search across entry names and tag names.
    /// Case-insensitive. Returns active entries only (trashed skipped).
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.all_active();
        }
        self.entries
            .values()
            .filter(|e| !e.is_trashed)
            .filter(|e| self.matches_query(e, &q))
            .collect()
    }

    fn matches_query(&self, e: &IndexEntry, q: &str) -> bool {
        if e.name.to_lowercase().contains(q) {
            return true;
        }
        if let Some(url) = e.url.as_deref() {
            if url.to_lowercase().contains(q) {
                return true;
            }
        }
        e.tag_ids
            .iter()
            .any(|tid| self.tags.get(tid).is_some_and(|t| t.name.contains(q)))
    }

    /// Entries carrying `tag_id`. Returns active and trashed both — callers
    /// filter. Empty list if the tag doesn't exist.
    #[must_use]
    pub fn by_tag(&self, tag_id: &TagId) -> Vec<&IndexEntry> {
        self.tag_index
            .get(tag_id)
            .into_iter()
            .flatten()
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Direct children of a folder (non-recursive). `None` = root-level entries.
    #[must_use]
    pub fn by_folder(&self, folder_id: Option<&EntryId>) -> Vec<&IndexEntry> {
        self.entries
            .values()
            .filter(|e| !e.is_trashed)
            .filter(|e| e.folder_id.as_ref() == folder_id)
            .collect()
    }

    /// Entries whose URL's registrable domain matches `domain` (case-insensitive).
    #[must_use]
    pub fn by_domain(&self, domain: &str) -> Vec<&IndexEntry> {
        let key = domain.to_lowercase();
        self.url_index
            .get(&key)
            .into_iter()
            .flatten()
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    #[must_use]
    pub fn all_active(&self) -> Vec<&IndexEntry> {
        self.entries.values().filter(|e| !e.is_trashed).collect()
    }

    #[must_use]
    pub fn all_trashed(&self) -> Vec<&IndexEntry> {
        self.entries.values().filter(|e| e.is_trashed).collect()
    }

    // ---- Mutations (used by use cases, starting Phase 4) -------------------
    // These are `pub(crate)` and called directly by the tests below. Phase 4+
    // use cases will call them from inside the `use_cases` module.
    pub(crate) fn insert_entry(&mut self, entry: IndexEntry) {
        let id = entry.id.clone();
        self.attach_derived(&entry);
        self.entries.insert(id, entry);
    }

    pub(crate) fn remove_entry(&mut self, id: &EntryId) {
        if let Some(existing) = self.entries.shift_remove(id) {
            self.detach_derived(&existing);
        }
    }

    pub(crate) fn update_entry(&mut self, entry: IndexEntry) {
        if let Some(existing) = self.entries.get(&entry.id) {
            // Rebuild derived indexes by detaching the old snapshot first,
            // then attaching the new one. Prevents stale tag/url refs from
            // leaking through when fields change.
            let old = existing.clone();
            self.detach_derived(&old);
        }
        self.attach_derived(&entry);
        let id = entry.id.clone();
        self.entries.insert(id, entry);
    }

    pub(crate) fn insert_tag(&mut self, tag: TagMeta) {
        self.tag_index.entry(tag.id.clone()).or_default();
        self.tags.insert(tag.id.clone(), tag);
    }

    pub(crate) fn rename_tag(&mut self, id: &TagId, new_name: String) {
        if let Some(t) = self.tags.get_mut(id) {
            t.name = new_name;
        }
    }

    pub(crate) fn remove_tag(&mut self, id: &TagId) {
        self.tags.remove(id);
        self.tag_index.remove(id);
        // The caller is responsible for also removing the tag_id from any
        // entries that carried it — we don't mutate entries here.
    }

    // ---- Derived-index bookkeeping ------------------------------------------

    fn attach_derived(&mut self, entry: &IndexEntry) {
        for tid in &entry.tag_ids {
            self.tag_index
                .entry(tid.clone())
                .or_default()
                .push(entry.id.clone());
        }
        if let Some(dom) = entry.url.as_deref().and_then(registrable_domain) {
            self.url_index
                .entry(dom)
                .or_default()
                .push(entry.id.clone());
        }
        if let Some(folder) = entry.folder_id.clone() {
            self.folders
                .entry(folder)
                .or_default()
                .push(entry.id.clone());
        }
    }

    fn detach_derived(&mut self, entry: &IndexEntry) {
        for tid in &entry.tag_ids {
            if let Some(v) = self.tag_index.get_mut(tid) {
                v.retain(|x| x != &entry.id);
            }
        }
        if let Some(dom) = entry.url.as_deref().and_then(registrable_domain) {
            if let Some(v) = self.url_index.get_mut(&dom) {
                v.retain(|x| x != &entry.id);
            }
        }
        if let Some(folder) = entry.folder_id.as_ref() {
            if let Some(v) = self.folders.get_mut(folder) {
                v.retain(|x| x != &entry.id);
            }
        }
    }
}

impl Default for VaultIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for VaultIndex {
    fn zeroize(&mut self) {
        // Name / URL / tag-name strings aren't secrets but we clear them on
        // lock anyway — the post-lock heap should carry no vault-identifying
        // residue. `Vec::clear` + `String::clear` free the backing buffers via
        // Drop; we also call zeroize on the inner Strings for the belt-and-
        // suspenders cases where the allocator may reuse the same region.
        for (_, e) in self.entries.drain(..) {
            zeroize_entry(e);
        }
        for (_, t) in self.tags.drain() {
            zeroize_tag(t);
        }
        self.tag_index.clear();
        self.url_index.clear();
        self.folders.clear();
    }
}

fn zeroize_entry(mut e: IndexEntry) {
    e.name.zeroize();
    if let Some(mut u) = e.url.take() {
        u.zeroize();
    }
    if let Some(mut u) = e.favicon_url.take() {
        u.zeroize();
    }
    if let Some(mut c) = e.color.take() {
        c.zeroize();
    }
    if let Some(mut ic) = e.icon.take() {
        ic.zeroize();
    }
}

fn zeroize_tag(mut t: TagMeta) {
    t.name.zeroize();
    if let Some(mut c) = t.color.take() {
        c.zeroize();
    }
}

/// Best-effort registrable-domain extraction.
///
/// Naïve — takes the rightmost two labels of the host after stripping scheme,
/// port, path, query and `www.`. Good enough for browser-extension autofill
/// matching across the common gTLD/ccTLD cases. Explicitly **not** a public-
/// suffix-list lookup — `co.uk` style two-part suffixes will over-match.
/// Bringing in `publicsuffix` is a future concern.
#[must_use]
pub fn registrable_domain(url: &str) -> Option<String> {
    let mut rest = url.trim();
    for scheme in ["https://", "http://"] {
        if let Some(stripped) = rest.strip_prefix(scheme) {
            rest = stripped;
            break;
        }
    }
    // Trim user info, path, query, fragment.
    if let Some(i) = rest.find(['/', '?', '#']) {
        rest = &rest[..i];
    }
    if let Some(i) = rest.rfind('@') {
        rest = rest.get(i.saturating_add(1)..).unwrap_or("");
    }
    if let Some(i) = rest.rfind(':') {
        rest = rest.get(..i).unwrap_or("");
    }
    if rest.is_empty() {
        return None;
    }
    let host = rest.strip_prefix("www.").unwrap_or(rest).to_lowercase();
    let parts: Vec<&str> = host.split('.').filter(|s| !s.is_empty()).collect();
    match parts.as_slice() {
        [] => None,
        [sole] => Some((*sole).to_owned()),
        [.., second_last, last] => Some(format!("{second_last}.{last}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::shared::now;
    use crate::domain::vault::entities::EntryRow;
    use crate::domain::vault::payloads::{CommonMeta, EntryType, FolderPayload, NotePayload};
    use secrecy::SecretString;

    fn fake_row(id: &EntryId) -> EntryRow {
        EntryRow {
            id: id.clone(),
            version: 1,
            cipher_suite: 1,
            dek_wrapped: [0u8; 40],
            nonce: [0u8; 24],
            ciphertext: Vec::new(),
            created_at: now(),
            updated_at: now(),
            accessed_at: None,
            is_trashed: false,
            trashed_at: None,
        }
    }

    fn note_payload(name: &str, url: Option<&str>, tags: Vec<TagId>) -> EntryPayload {
        let mut meta = CommonMeta::new(name, EntryType::Note);
        meta.url = url.map(str::to_owned);
        meta.tag_ids = tags;
        EntryPayload::Note(NotePayload {
            meta,
            content: SecretString::from("x"),
        })
    }

    fn folder_payload(name: &str) -> EntryPayload {
        EntryPayload::Folder(FolderPayload {
            meta: CommonMeta::new(name, EntryType::Folder),
        })
    }

    #[test]
    fn registrable_domain_handles_common_cases() {
        assert_eq!(
            registrable_domain("https://github.com/settings/tokens").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            registrable_domain("http://www.github.com").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            registrable_domain("http://user:pass@example.org:8443/path?q=1#f").as_deref(),
            Some("example.org")
        );
        assert_eq!(
            registrable_domain("localhost").as_deref(),
            Some("localhost")
        );
        assert_eq!(registrable_domain(""), None);
    }

    #[test]
    fn insert_update_remove_entry_keeps_derived_consistent() {
        let mut idx = VaultIndex::new();
        let tag_a = TagId::new();
        idx.insert_tag(TagMeta {
            id: tag_a.clone(),
            name: "github".into(),
            color: None,
            sort_order: 0,
        });

        let entry_id = EntryId::new();
        let row = fake_row(&entry_id);
        let p = note_payload(
            "github token",
            Some("https://github.com"),
            vec![tag_a.clone()],
        );
        idx.insert_entry(IndexEntry::from_payload(&p, &row));

        assert_eq!(idx.all_active().len(), 1);
        assert_eq!(idx.by_tag(&tag_a).len(), 1);
        assert_eq!(idx.by_domain("github.com").len(), 1);
        assert!(idx.by_domain("example.com").is_empty());

        // Update — drop the URL. Derived indexes must clean up.
        let p2 = note_payload("github token", None, vec![tag_a.clone()]);
        let mut row2 = row;
        row2.updated_at = now();
        idx.update_entry(IndexEntry::from_payload(&p2, &row2));
        assert!(idx.by_domain("github.com").is_empty());
        assert_eq!(idx.by_tag(&tag_a).len(), 1);

        // Remove.
        idx.remove_entry(&entry_id);
        assert!(idx.all_active().is_empty());
        assert!(idx.by_tag(&tag_a).is_empty());
    }

    #[test]
    fn search_matches_name_url_and_tag_names() {
        let mut idx = VaultIndex::new();
        let t = TagId::new();
        idx.insert_tag(TagMeta {
            id: t.clone(),
            name: "aws".into(),
            color: None,
            sort_order: 0,
        });
        let e1 = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &note_payload("prod", Some("https://console.aws.amazon.com"), vec![t]),
            &fake_row(&e1),
        ));
        let e2 = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &note_payload("random", None, vec![]),
            &fake_row(&e2),
        ));

        assert_eq!(idx.search("prod").len(), 1);
        assert_eq!(idx.search("aws").len(), 1); // via tag name match
        assert_eq!(idx.search("random").len(), 1);
        assert_eq!(idx.search("none").len(), 0);
    }

    #[test]
    fn from_payload_projects_color_and_icon() {
        let mut meta = CommonMeta::new("Work", EntryType::Folder);
        meta.color = Some("#ef4444".into());
        meta.icon = Some("briefcase".into());
        let payload = EntryPayload::Folder(FolderPayload { meta });
        let e = IndexEntry::from_payload(&payload, &fake_row(&EntryId::new()));
        assert_eq!(e.color.as_deref(), Some("#ef4444"));
        assert_eq!(e.icon.as_deref(), Some("briefcase"));
    }

    #[test]
    fn by_folder_filters_children() {
        let mut idx = VaultIndex::new();
        let folder = EntryId::new();
        idx.insert_entry(IndexEntry::from_payload(
            &folder_payload("Work"),
            &fake_row(&folder),
        ));
        let child = EntryId::new();
        let mut p = note_payload("item", None, vec![]);
        p.meta_mut().folder_id = Some(folder.clone());
        idx.insert_entry(IndexEntry::from_payload(&p, &fake_row(&child)));

        assert_eq!(idx.by_folder(Some(&folder)).len(), 1);
        assert_eq!(idx.by_folder(None).len(), 1); // the folder itself is root-level
    }

    #[test]
    fn trashed_entries_bucket_separately() {
        let mut idx = VaultIndex::new();
        let id = EntryId::new();
        let mut row = fake_row(&id);
        row.is_trashed = true;
        idx.insert_entry(IndexEntry::from_payload(
            &note_payload("x", None, vec![]),
            &row,
        ));
        assert!(idx.all_active().is_empty());
        assert_eq!(idx.all_trashed().len(), 1);
    }

    #[test]
    fn zeroize_clears_all_maps() {
        let mut idx = VaultIndex::new();
        let t = TagId::new();
        idx.insert_tag(TagMeta {
            id: t.clone(),
            name: "aws".into(),
            color: None,
            sort_order: 0,
        });
        idx.insert_entry(IndexEntry::from_payload(
            &note_payload("x", Some("https://example.com"), vec![t]),
            &fake_row(&EntryId::new()),
        ));
        idx.zeroize();
        assert!(idx.entries.is_empty());
        assert!(idx.tags.is_empty());
        assert!(idx.tag_index.is_empty());
        assert!(idx.url_index.is_empty());
        assert!(idx.folders.is_empty());
    }
}
