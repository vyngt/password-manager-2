use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::vault_create_form::VaultCreateForm;
use crate::features::vault::vault_detail::VaultDetail;
use crate::features::vault::vault_search::VaultSearch;
use crate::features::vault::vault_table::VaultTable;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{FieldSelectorDto, IndexEntryDto};
use vedge_ui::components::Button;
use vedge_ui::primitives::tokens::{Size, Variant};

/// Client-side filter predicate over the loaded metadata. Matches on `name`
/// and `url` (case-insensitive). Secrets/`username` aren't in `IndexEntryDto`,
/// so those are not searchable client-side — server-side `search` is the
/// scale follow-up.
fn matches(entry: &IndexEntryDto, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    entry.name.to_lowercase().contains(&q)
        || entry
            .url
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(&q)
}

#[component]
pub fn VaultPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let items = RwSignal::new(Vec::<IndexEntryDto>::new());
    let loading = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());
    let show_create_form = RwSignal::new(false);
    let selected = RwSignal::new(Option::<IndexEntryDto>::None);
    let status_msg = RwSignal::new(Option::<String>::None);

    // Fetch the live entry index for the active vault. Re-run on demand
    // (mount, create-ping, post-delete).
    let refresh = move || {
        let vault_path = active.path.get().unwrap_or_default();
        if vault_path.is_empty() {
            items.set(Vec::new());
            return;
        }
        loading.set(true);
        // `refresh` is called from an Effect *and* from inside `spawn_local`
        // (on delete); `untrack` reads the current locale string safely in both.
        let err_prefix = untrack(|| t_string!(i18n, vault.err_load).to_string());
        spawn_local(async move {
            match api::vault::list_entries(&vault_path).await {
                Ok(list) => items.set(list),
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
            loading.set(false);
        });
    };

    // Load on mount and whenever the active vault path changes.
    Effect::new(move |_| {
        let _ = active.path.get();
        refresh();
    });

    let on_delete = Callback::new(move |id: String| {
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_delete).to_string();
        spawn_local(async move {
            match api::entry::soft_delete_entry(&vault_path, &id).await {
                Ok(()) => {
                    if selected.get().map(|e| e.id).as_deref() == Some(id.as_str()) {
                        selected.set(None);
                    }
                    refresh();
                }
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    });

    let on_select = Callback::new(move |entry: IndexEntryDto| selected.set(Some(entry)));
    let on_close = Callback::new(move |()| selected.set(None));
    let on_created = Callback::new(move |()| refresh());

    let on_copy = Callback::new(move |field: FieldSelectorDto| {
        let vault_path = active.path.get().unwrap_or_default();
        let Some(id) = selected.get().map(|e| e.id) else {
            return;
        };
        // Read the locale string here (reactive owner present); reading it
        // inside `spawn_local` trips the "outside a reactive tracking context"
        // warning.
        let copied_msg = t_string!(i18n, vault.copied).to_string();
        let err_prefix = t_string!(i18n, vault.err_copy).to_string();
        spawn_local(async move {
            match api::entry::copy_field(&vault_path, &id, &field, Some(30)).await {
                Ok(()) => status_msg.set(Some(copied_msg)),
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    });

    view! {
        <div class="h-full flex flex-col gap-4 p-4">
            <div class="flex items-center gap-3">
                <VaultSearch search_query=search_query />
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    on:click=move |_| show_create_form.update(|v| *v = !*v)
                >
                    {move || t!(i18n, vault.new_item)}
                </Button>
            </div>

            {move || status_msg.get().map(|m| view! {
                <p class="text-sm text-text-secondary">{m}</p>
            })}

            <Show when=move || show_create_form.get()>
                <VaultCreateForm show=show_create_form on_created=on_created />
            </Show>

            <div class="flex-1 flex gap-4 min-h-0">
                <Show
                    when=move || !loading.get()
                    fallback=move || {
                        view! {
                            <div class="flex-1 flex items-center justify-center text-foreground/40 text-sm">
                                {move || t!(i18n, vault.loading)}
                            </div>
                        }
                    }
                >
                    <VaultTable
                        items=Signal::derive(move || {
                            items.get().into_iter().filter(|e| matches(e, &search_query.get())).collect::<Vec<_>>()
                        })
                        on_delete=on_delete
                        on_select=on_select
                    />
                </Show>
                {move || selected.get().map(|entry| view! {
                    <VaultDetail entry=entry on_copy=on_copy on_close=on_close />
                })}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::matches;
    use vedge_ipc::{EntryTypeDto, IndexEntryDto};

    fn entry(name: &str, url: Option<&str>) -> IndexEntryDto {
        IndexEntryDto {
            id: "1".into(),
            name: name.into(),
            entry_type: EntryTypeDto::Login,
            url: url.map(str::to_string),
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            is_trashed: false,
            cipher_suite: 1,
            created_at: "2026-07-05T00:00:00.000Z".into(),
            updated_at: "2026-07-05T00:00:00.000Z".into(),
            accessed_at: None,
        }
    }

    #[test]
    fn empty_query_matches_all() {
        assert!(matches(&entry("GitHub", Some("https://github.com")), "  "));
    }

    #[test]
    fn matches_name_case_insensitive() {
        assert!(matches(&entry("GitHub", None), "hub"));
        assert!(!matches(&entry("GitHub", None), "gitlab"));
    }

    #[test]
    fn matches_url() {
        assert!(matches(&entry("x", Some("https://example.com")), "example"));
    }

    #[test]
    fn no_url_does_not_panic() {
        assert!(!matches(&entry("x", None), "example"));
    }
}
