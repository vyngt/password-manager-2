//! Entry detail panel. Renders **metadata only** (`IndexEntryDto` carries no
//! secrets) plus copy actions. Copying a secret goes through the backend
//! `copy_field` command (`on_copy`) which puts it on the OS clipboard with a
//! 30 s auto-clear — plaintext never enters the renderer.

use super::entry_view::{short_date, type_label};
use crate::i18n::*;
use leptos::prelude::*;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, IndexEntryDto};
use vedge_ui::components::Button;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

#[component]
pub fn VaultDetail(
    entry: IndexEntryDto,
    on_copy: Callback<FieldSelectorDto>,
    on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    let is_login = entry.entry_type == EntryTypeDto::Login;
    let name = entry.name.clone();
    let type_lbl = type_label(&entry.entry_type);
    let url = entry.url.clone();
    let updated = short_date(&entry.updated_at);
    let created = short_date(&entry.created_at);

    view! {
        <aside class="w-80 shrink-0 border border-border rounded-lg p-4 bg-primary-muted overflow-auto">
            <div class="flex items-center justify-between mb-3">
                <h3 class="text-sm font-semibold text-text-secondary">
                    {move || t!(i18n, vault.detail_title)}
                </h3>
                <IconButton
                    aria_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
                    variant=Variant::Ghost
                    size=Size::Sm
                    on:click=move |_: web_sys::MouseEvent| on_close.run(())
                >
                    <span aria-hidden="true">"✕"</span>
                </IconButton>
            </div>

            <dl class="flex flex-col gap-2 text-sm">
                <div>
                    <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                        {move || t!(i18n, vault.col_name)}
                    </dt>
                    <dd class="text-text-primary">{name}</dd>
                </div>
                <div>
                    <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                        {move || t!(i18n, vault.col_type)}
                    </dt>
                    <dd class="text-text-primary">{type_lbl}</dd>
                </div>
                {url.map(|u| {
                    let href = u.clone();
                    view! {
                        <div>
                            <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                {move || t!(i18n, vault.col_url)}
                            </dt>
                            <dd>
                                <a
                                    class="text-primary hover:underline font-jetbrains-mono break-all"
                                    href=href
                                    target="_blank"
                                    rel="noreferrer"
                                >
                                    {u}
                                </a>
                            </dd>
                        </div>
                    }
                })}
                <div>
                    <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                        {move || t!(i18n, vault.col_updated)}
                    </dt>
                    <dd class="text-foreground/70">{updated}</dd>
                </div>
                <div>
                    <dt class="text-foreground/50 text-xs uppercase tracking-wider">"Created"</dt>
                    <dd class="text-foreground/70">{created}</dd>
                </div>
            </dl>

            {is_login.then(|| view! {
                <div class="flex flex-col gap-2 mt-4">
                    <Button
                        variant=Variant::Secondary
                        size=Size::Sm
                        full_width=true
                        on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::Username)
                    >
                        {move || t!(i18n, vault.copy_username)}
                    </Button>
                    <Button
                        variant=Variant::Primary
                        size=Size::Sm
                        full_width=true
                        on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::Password)
                    >
                        {move || t!(i18n, vault.copy_password)}
                    </Button>
                </div>
            })}
        </aside>
    }
}
