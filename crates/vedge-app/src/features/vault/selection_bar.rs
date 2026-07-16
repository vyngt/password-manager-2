//! The multi-select **action bar** (slice 5.3.1c, decision ⑥). When the
//! selection is non-empty the vault toolbar is *replaced* by this bar — it is a
//! mode, not a stacked strip. Verbs act on the current selection and reuse the
//! single-entry operations (there are no bulk commands; see `vault.rs`).
//!
//! `role="toolbar"` + an `aria-live="polite"` count make the mode announced;
//! `Esc` (here, and from inside the grid) clears the selection.
//!
//! Export… and the whole-vault "select all N" escalation are **not** here — they
//! land in PR d with the three-scope export dialog (deferring them keeps the
//! selection ⊆ the filter, which is what makes ⑤ safe).

use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::{Button, IconButton};
use vedge_ui::primitives::tokens::{Size, Variant};

#[component]
pub fn SelectionBar(
    /// How many entries are selected (drives the live count).
    #[prop(into)]
    count: Signal<usize>,
    /// Open the bulk add-tags dialog.
    on_tag: Callback<()>,
    /// Open the bulk move-to-folder dialog.
    on_move: Callback<()>,
    /// Open the bulk Trash confirm.
    on_trash: Callback<()>,
    /// Clear the selection (also the ✕ button and `Esc`).
    on_clear: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div
            role="toolbar"
            aria-label=move || t_string!(i18n, vault.selection_toolbar_aria).to_owned()
            class="flex-1 flex items-center gap-3 min-w-0"
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    on_clear.run(());
                }
            }
        >
            <span
                aria-live="polite"
                class="text-sm font-medium text-text-primary whitespace-nowrap"
                data-testid="selection-count"
            >
                {move || {
                    let n = count.get();
                    t!(i18n, vault.selected_count, count = n)
                }}
            </span>
            <div class="flex items-center gap-2">
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    attr:data-testid="bulk-tag"
                    on:click=move |_| on_tag.run(())
                >
                    {move || t!(i18n, vault.bulk_tag)}
                </Button>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    attr:data-testid="bulk-move"
                    on:click=move |_| on_move.run(())
                >
                    {move || t!(i18n, vault.bulk_move)}
                </Button>
                <Button
                    variant=Variant::Danger
                    size=Size::Sm
                    class="whitespace-nowrap"
                    attr:data-testid="bulk-trash"
                    on:click=move |_| on_trash.run(())
                >
                    {move || t!(i18n, vault.bulk_trash)}
                </Button>
            </div>
            <IconButton
                aria_label=Signal::derive(move || t_string!(i18n, vault.selection_clear).to_owned())
                variant=Variant::Ghost
                size=Size::Sm
                class="ml-auto"
                attr:data-testid="selection-clear"
                on:click=move |_: web_sys::MouseEvent| on_clear.run(())
            >
                <Icon attr:aria-hidden="true" icon=i::FaXmarkSolid />
            </IconButton>
        </div>
    }
}
