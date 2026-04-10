use super::types::{VaultItem, VaultItemData};
use crate::i18n::*;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use leptos::prelude::*;
use reactive_stores::Store;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Effect as ButtonEffect, Shape, Size, Variant};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn VaultTable(
    #[prop(into)] items: Signal<Vec<VaultItem>>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex-1 overflow-auto">
            <Show
                when=move || !items.get().is_empty()
                fallback=move || {
                    view! {
                        <div class="flex items-center justify-center h-full text-foreground/40 text-sm">
                            {move || t!(i18n, vault.no_items)}
                        </div>
                    }
                }
            >
                <table class="w-full table-auto">
                    <thead>
                        <tr class="border-b border-secondary/20 text-left text-foreground/60 text-xs uppercase tracking-wider">
                            <th class="p-3">{move || t!(i18n, vault.col_title)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_identifier)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_url)}</th>
                            <th class="p-3 w-[80px]">{move || t!(i18n, vault.col_actions)}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || items.get()
                            key=|item| item.id.clone()
                            children=move |item| {
                                view! { <VaultTableRow item=item on_delete=on_delete /> }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

#[component]
fn VaultTableRow(item: VaultItem, on_delete: Callback<String>) -> impl IntoView {
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    let (identifier, url) = match &item.data {
        VaultItemData::Credential(cred) => (cred.identifier.clone(), cred.url.clone()),
    };

    let item_id = item.id.clone();

    view! {
        <tr class="border-b border-secondary/10 hover:bg-primary/5 transition-colors">
            <td class="p-3 text-sm">{item.title}</td>
            <td class="p-3 text-sm font-jetbrains-mono text-foreground/80">{identifier}</td>
            <td class="p-3 text-sm font-jetbrains-mono text-foreground/60">{url}</td>
            <td class="p-3">
                <IconButton
                    color=Signal::derive(move || color_store.danger().get())
                    variant=Variant::Text
                    size=Size::Small
                    shape=Shape::Rounded
                    effect=ButtonEffect::Ripple
                    class="p-1"
                    on:click=move |_| on_delete.run(item_id.clone())
                >
                    <Icon icon=i::BiTrashRegular />
                </IconButton>
            </td>
        </tr>
    }
}
