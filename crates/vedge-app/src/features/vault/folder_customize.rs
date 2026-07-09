//! Folder customize dialog: pick a color + icon for a folder. Opened from the
//! tree's palette button (driven by a `target` signal). Because a folder carries
//! no secrets, the page persists the choice with a plain `get_entry →
//! update_entry` round-trip (see `vault.rs::on_customize`).

use super::folder_tree::{FOLDER_ICONS, folder_icon_from_key};
use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ipc::IndexEntryDto;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::form::color_picker::{ColorPicker, SwatchItem};
use vedge_ui::components::{Button, IconButton};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

/// Preset folder colors offered as `ColorPicker` swatches (also the default when
/// none is set yet).
const FOLDER_COLOR_PRESETS: &[&str] = &[
    "#4f46e5", "#0ea5e9", "#10b981", "#f59e0b", "#ef4444", "#ec4899", "#8b5cf6", "#64748b",
];
const DEFAULT_PICK: &str = "#4f46e5";

/// The customize dialog. Shown while `target` is `Some`; picks a color + icon and
/// fires `on_apply((folder_id, color, icon))`.
#[component]
pub fn FolderCustomize(
    target: RwSignal<Option<IndexEntryDto>>,
    /// `(folder_id, color, icon_key)` — persist the chosen presentation.
    on_apply: Callback<(String, Option<String>, Option<String>)>,
) -> impl IntoView {
    let i18n = use_i18n();
    let color = RwSignal::new(Option::<String>::None);
    let icon = RwSignal::new(Option::<String>::None);

    // Seed from the folder's current presentation each time the dialog opens.
    Effect::new(move |_| {
        if let Some(e) = target.get() {
            color.set(e.color.clone());
            icon.set(e.icon.clone());
        }
    });

    let close = Callback::new(move |()| target.set(None));
    let confirm = move |_: web_sys::MouseEvent| {
        if let Some(e) = target.get_untracked() {
            on_apply.run((e.id, color.get_untracked(), icon.get_untracked()));
        }
        target.set(None);
    };

    // Held in a `StoredValue` (Copy) so the Dialog children closure stays `Fn` —
    // capturing the owned `Vec` directly would move it out and make it `FnOnce`.
    let swatches = StoredValue::new(
        FOLDER_COLOR_PRESETS
            .iter()
            .map(|c| SwatchItem {
                value: (*c).to_owned(),
                label: None,
            })
            .collect::<Vec<SwatchItem>>(),
    );

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.folder_customize_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-6 min-w-[22rem]">
                    // Live preview chip: the resolved icon tinted by the color.
                    {move || {
                        let name = target.get().map(|e| e.name).unwrap_or_default();
                        let icon_data = folder_icon_from_key(
                            icon.get().as_deref().unwrap_or_default(),
                        );
                        view! {
                            <div class="flex items-center gap-2 text-sm font-medium text-text-primary">
                                <span
                                    class="flex shrink-0 text-foreground/50"
                                    style:color=move || color.get().unwrap_or_default()
                                >
                                    <Icon
                                        attr:aria-hidden="true"
                                        icon=icon_data
                                        width="16"
                                        height="16"
                                    />
                                </span>
                                <span class="truncate">{name}</span>
                            </div>
                        }
                    }} // Color.
                    <div class="flex flex-col gap-1">
                        <span class="text-foreground/50 text-xs uppercase tracking-wider">
                            {move || t!(i18n, vault.folder_color)}
                        </span>
                        <div class="flex items-center gap-2">
                            <ColorPicker
                                value=Signal::derive(move || {
                                    color.get().unwrap_or_else(|| DEFAULT_PICK.to_owned())
                                })
                                swatches=swatches.get_value()
                                on_change_end=Callback::new(move |c: String| color.set(Some(c)))
                            />
                            <Button
                                variant=Variant::Ghost
                                size=Size::Sm
                                on:click=move |_: web_sys::MouseEvent| color.set(None)
                            >
                                {move || t!(i18n, vault.folder_color_clear)}
                            </Button>
                        </div>
                    // Icon grid.
                    </div> <div class="flex flex-col gap-1">
                        <span class="text-foreground/50 text-xs uppercase tracking-wider">
                            {move || t!(i18n, vault.folder_icon)}
                        </span>
                        <div class="grid grid-cols-8 gap-1">
                            {FOLDER_ICONS
                                .iter()
                                .map(|(key, ic)| {
                                    let key_s = (*key).to_owned();
                                    let sel_cls = key_s.clone();
                                    let sel_p = key_s.clone();
                                    let label = key_s.clone();
                                    let ic = *ic;
                                    // One clone per closure (the reactive selected-class read, the
                                    // aria-pressed read, and the click) since the key is non-Copy.
                                    // The icon key doubles as the accessible name — there's no
                                    // localized per-icon string set, so the identifier is the
                                    // best available label for this visual picker.
                                    view! {
                                        <IconButton
                                            variant=Variant::Ghost
                                            size=Size::Md
                                            aria_label=label
                                            attr:aria-pressed=move || {
                                                (icon.get().as_deref() == Some(sel_p.as_str()))
                                                    .then_some("true")
                                            }
                                            class=Signal::derive(move || {
                                                let base = "border border-border hover:border-primary text-foreground/70";
                                                if icon.get().as_deref() == Some(sel_cls.as_str()) {
                                                    format!("{base} border-primary text-primary bg-primary/10")
                                                } else {
                                                    base.to_string()
                                                }
                                            })
                                            on:click=move |_: web_sys::MouseEvent| {
                                                icon.set(Some(key_s.clone()));
                                            }
                                        >
                                            <Icon
                                                attr:aria-hidden="true"
                                                icon=ic
                                                width="16"
                                                height="16"
                                            />
                                        </IconButton>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div> <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        <Button variant=Variant::Primary size=Size::Sm on:click=confirm>
                            {move || t!(i18n, vault.folder_customize_apply)}
                        </Button>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
