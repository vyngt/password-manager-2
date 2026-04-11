use crate::api::tauri;
use crate::features::vault::types::{PaginationOutput, VaultItem, VaultItemData};
use crate::features::vault::vault_create_form::VaultCreateForm;
use crate::features::vault::vault_search::VaultSearch;
use crate::features::vault::vault_table::VaultTable;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use serde_wasm_bindgen::{from_value, to_value as to_js_value};
use ui::components::Button;
use ui::primitives::tokens::{Size, Variant};

fn load_items(items: RwSignal<Vec<VaultItem>>, loading: RwSignal<bool>) {
    loading.set(true);
    spawn_local(async move {
        let Some(args) = to_js_value(&json!({
            "request": {
                "pagination": { "limit": 100, "offset": 0 }
            }
        }))
        .ok() else {
            loading.set(false);
            return;
        };

        let res = tauri::invoke("list_vault_items", args).await;

        match from_value::<PaginationOutput<VaultItem>>(res) {
            Ok(output) => {
                items.set(output.data);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to load vault items: {:?}", e).into());
            }
        }

        loading.set(false);
    });
}

#[component]
pub fn VaultPage() -> impl IntoView {
    let i18n = use_i18n();

    let items = RwSignal::new(Vec::<VaultItem>::new());
    let loading = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());
    let show_create_form = RwSignal::new(false);

    // Load items on mount
    Effect::new(move |_| {
        load_items(items, loading);
    });

    // Client-side search filter
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

    // Delete handler
    let on_delete = Callback::new(move |id: String| {
        spawn_local(async move {
            let Some(args) = to_js_value(&json!({
                "request": { "id": id }
            }))
            .ok() else {
                return;
            };

            let res = tauri::invoke("delete_vault_item", args).await;

            match from_value::<VaultItem>(res) {
                Ok(deleted) => {
                    items.update(|list| {
                        list.retain(|item| item.id != deleted.id);
                    });
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to delete vault item: {:?}", e).into(),
                    );
                }
            }
        });
    });

    // Create handler
    let on_created = Callback::new(move |item: VaultItem| {
        items.update(|list| list.push(item));
    });

    view! {
        <div class="h-full flex flex-col gap-4 p-4">
            // Toolbar
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

            // Create form
            <Show when=move || show_create_form.get()>
                <VaultCreateForm show=show_create_form on_created=on_created />
            </Show>

            // Table
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
