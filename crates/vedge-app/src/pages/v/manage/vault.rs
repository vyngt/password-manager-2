use crate::features::vault::types::{VaultItem, VaultItemData};
use crate::features::vault::vault_create_form::VaultCreateForm;
use crate::features::vault::vault_search::VaultSearch;
use crate::features::vault::vault_table::VaultTable;
use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::Button;
use vedge_ui::primitives::tokens::{Size, Variant};

// TODO(UI adaptation): this page still uses the legacy `VaultItem` /
// `VaultItemData::Credential` shape. The real data source is
// `api::vault::list_entries(vault_path).await` returning
// `Vec<IndexEntryDto>`; delete goes through `api::entry::soft_delete_entry`.
// Component tree needs to be rebuilt around the new DTOs before this page
// can talk to the live shell. See docs/ImplementAppBridge.md "What comes
// after this plan".
fn load_items(items: RwSignal<Vec<VaultItem>>, loading: RwSignal<bool>) {
    loading.set(true);
    items.set(Vec::new());
    loading.set(false);
}

#[component]
pub fn VaultPage() -> impl IntoView {
    let i18n = use_i18n();

    let items = RwSignal::new(Vec::<VaultItem>::new());
    let loading = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());
    let show_create_form = RwSignal::new(false);

    Effect::new(move |_| {
        load_items(items, loading);
    });

    let filtered_items = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        if query.is_empty() {
            return items.get();
        }
        items
            .get()
            .into_iter()
            .filter(|item| {
                let matches_title = item.title.to_lowercase().contains(&query);
                let matches_data = match &item.data {
                    VaultItemData::Credential(cred) => {
                        cred.identifier.to_lowercase().contains(&query)
                            || cred.url.to_lowercase().contains(&query)
                    }
                };
                matches_title || matches_data
            })
            .collect::<Vec<_>>()
    });

    // TODO(UI adaptation): wire to `api::entry::soft_delete_entry`.
    let on_delete = Callback::new(move |id: String| {
        items.update(|list| list.retain(|item| item.id != id));
    });

    // 1.4: the form now persists through the backend and pings us to refresh.
    // `load_items` is still the legacy stub here; it becomes the real
    // `api::vault::list_entries` fetch in slice 1.5.
    let on_created = Callback::new(move |()| load_items(items, loading));

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

            <Show when=move || show_create_form.get()>
                <VaultCreateForm show=show_create_form on_created=on_created />
            </Show>

            <Show
                when=move || !loading.get()
                fallback=|| {
                    view! {
                        <div class="flex-1 flex items-center justify-center text-foreground/40 text-sm">
                            "Loading..."
                        </div>
                    }
                }
            >
                <VaultTable
                    items=Signal::derive(move || filtered_items.get())
                    on_delete=on_delete
                />
            </Show>
        </div>
    }
}
