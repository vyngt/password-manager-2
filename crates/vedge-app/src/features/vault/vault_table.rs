use super::entry_view::{short_date, type_label};
use crate::i18n::*;
use leptos::prelude::*;
use vedge_ipc::IndexEntryDto;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn VaultTable(
    #[prop(into)] items: Signal<Vec<IndexEntryDto>>,
    on_delete: Callback<String>,
    on_select: Callback<IndexEntryDto>,
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
                            <th class="p-3">{move || t!(i18n, vault.col_name)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_type)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_url)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_updated)}</th>
                            <th class="p-3 w-[80px]">{move || t!(i18n, vault.col_actions)}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || items.get()
                            key=|item| item.id.clone()
                            children=move |item| {
                                view! {
                                    <VaultTableRow
                                        item=item
                                        on_delete=on_delete
                                        on_select=on_select
                                    />
                                }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

#[component]
fn VaultTableRow(
    item: IndexEntryDto,
    on_delete: Callback<String>,
    on_select: Callback<IndexEntryDto>,
) -> impl IntoView {
    let i18n = use_i18n();

    let entry_for_select = item.clone();
    let item_id = item.id.clone();
    let name = item.name.clone();
    let type_lbl = type_label(&item.entry_type);
    let url = item.url.clone().unwrap_or_default();
    let updated = short_date(&item.updated_at);

    view! {
        <tr
            class="border-b border-secondary/10 hover:bg-primary/5 transition-colors cursor-pointer"
            on:click=move |_: web_sys::MouseEvent| on_select.run(entry_for_select.clone())
        >
            <td class="p-3 text-sm">{name}</td>
            <td class="p-3 text-sm text-foreground/70">{type_lbl}</td>
            <td class="p-3 text-sm font-jetbrains-mono text-foreground/60">{url}</td>
            <td class="p-3 text-sm text-foreground/60">{updated}</td>
            <td class="p-3">
                <IconButton
                    aria_label=Signal::derive(move || t_string!(i18n, vault.delete).to_string())
                    variant=Variant::Danger
                    size=Size::Sm
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        on_delete.run(item_id.clone());
                    }
                >
                    <Icon icon=i::BiTrashRegular />
                </IconButton>
            </td>
        </tr>
    }
}
