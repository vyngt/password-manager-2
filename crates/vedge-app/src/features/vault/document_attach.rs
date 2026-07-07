//! Attach-a-document panel. `Document` is the one entry type the field form
//! can't build (it's a file + encrypted blob sidecar), so it gets its own flow:
//! pick a file via the native open dialog, confirm a display **name** (prefilled
//! from the filename — Documents aren't editable afterward, so this is the only
//! chance to name it), then hand the *path* to the backend
//! (`api::document::import_document_from_path`). The file bytes are read and
//! encrypted entirely in the backend; nothing decrypted crosses into WASM.

use crate::api;
use crate::api::dialog::OpenDialogOptions;
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{CommonMetaDto, EntryTypeDto};
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::primitives::tokens::{Size, Variant};

/// Final path component (basename), splitting on both `/` and `\` so it works
/// for native paths on every platform.
fn file_basename(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned()
}

/// Build the metadata for a new `Document` entry. `entry_type` is forced to
/// `Document` (the backend also forces it); tags/folder are left at defaults —
/// assignment widgets land in 2.4/2.5.
fn document_meta(name: String) -> CommonMetaDto {
    CommonMetaDto {
        name,
        entry_type: EntryTypeDto::Document,
        url: None,
        favicon_url: None,
        tag_ids: vec![],
        folder_id: None,
        is_favorite: false,
        notes: None,
        color: None,
        icon: None,
    }
}

#[component]
pub fn DocumentAttach(show: RwSignal<bool>, on_attached: Callback<()>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let src_path = RwSignal::new(Option::<String>::None);
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let reset = move || {
        src_path.set(None);
        name.set(String::new());
        error.set(None);
        submitting.set(false);
    };

    let handle_cancel = move |_| {
        reset();
        show.set(false);
    };

    let choose_file = move |_| {
        // Read the locale string in the handler body (reactive owner present);
        // reading it inside `spawn_local` would warn.
        let dialog_title = t_string!(i18n, vault.attach_title).to_string();
        error.set(None);
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(dialog_title),
                filters: vec![],
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    name.set(file_basename(&path));
                    src_path.set(Some(path));
                }
                Ok(None) => {}
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    let handle_attach = move |_| {
        if submitting.get() {
            return;
        }
        let Some(path) = src_path.get() else {
            return;
        };
        let entry_name = name.get().trim().to_owned();
        if entry_name.is_empty() {
            return;
        }
        // Hoist every locale read out of the async block.
        let too_large = t_string!(i18n, vault.err_document_too_large).to_string();
        let err_prefix = t_string!(i18n, vault.err_import).to_string();
        let vault_path = active.path.get().unwrap_or_default();

        submitting.set(true);
        error.set(None);
        spawn_local(async move {
            let meta = document_meta(entry_name);
            match api::document::import_document_from_path(&vault_path, &path, &meta).await {
                Ok(_id) => {
                    reset();
                    show.set(false);
                    on_attached.run(());
                }
                // `DocumentTooLarge` collapses to `Invalid` on the wire; surface
                // the friendly, localized message for that specific case.
                Err(ApiError::Invalid(m)) if m.contains("too large") => error.set(Some(too_large)),
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            submitting.set(false);
        });
    };

    view! {
        <div class="border border-border rounded-lg p-4 bg-primary-muted">
            <h3 class="text-sm font-semibold mb-3 text-text-secondary">
                {move || t!(i18n, vault.attach_title)}
            </h3>

            <div class="flex items-center gap-3 mb-3">
                <Button variant=Variant::Secondary size=Size::Sm on:click=choose_file>
                    {move || t!(i18n, vault.attach_choose_file)}
                </Button>
                {move || src_path.get().map(|p| view! {
                    <span class="text-sm text-text-secondary font-jetbrains-mono break-all">
                        {file_basename(&p)}
                    </span>
                })}
            </div>

            <Show when=move || src_path.get().is_some()>
                <div class="mb-3">
                    <Input
                        id="doc-attach-name"
                        placeholder=Signal::derive(move || t_string!(i18n, vault.form_title).to_string())
                        value=Signal::derive(move || name.get())
                        on_input=Callback::new(move |v: String| name.set(v))
                    />
                </div>
            </Show>

            {move || error.get().map(|e| view! {
                <p class="text-sm mt-3" style="color:var(--color-danger-text)">{e}</p>
            })}

            <div class="flex gap-2 justify-end mt-3">
                <Button variant=Variant::Ghost size=Size::Sm on:click=handle_cancel>
                    {move || t!(i18n, vault.cancel)}
                </Button>
                {move || {
                    let busy = submitting.get()
                        || src_path.get().is_none()
                        || name.with(|n| n.trim().is_empty());
                    view! {
                        <Button
                            variant=Variant::Primary
                            size=Size::Sm
                            disabled=busy
                            on:click=handle_attach
                        >
                            {move || t!(i18n, vault.attach_confirm)}
                        </Button>
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::file_basename;

    #[test]
    fn basename_posix() {
        assert_eq!(file_basename("/home/me/report.pdf"), "report.pdf");
    }

    #[test]
    fn basename_windows() {
        assert_eq!(file_basename(r"C:\Users\me\key.txt"), "key.txt");
    }

    #[test]
    fn basename_bare() {
        assert_eq!(file_basename("notes.md"), "notes.md");
    }
}
