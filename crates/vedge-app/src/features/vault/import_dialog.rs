//! The Import… dialog (slice 5.3.1d, decision ①): the Settings Import panel
//! surfaced on the vault page, opened from the `⋯` menu. Import now lives where
//! the entries do — Settings ▸ Backup keeps its own copy of the same panel.
//!
//! This is a thin `Dialog` shell around the shared [`ImportPanel`]; on a
//! successful commit the panel pings `on_imported` so the vault list refreshes
//! behind the still-open report.

use leptos::prelude::*;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::primitives::tokens::DialogSize;

use crate::features::settings::import_panel::ImportPanel;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn ImportDialog(
    /// Drives the dialog's open state (set by the `⋯` "Import…" item).
    open: RwSignal<bool>,
    /// Fired after a successful commit so the host refreshes its entry list.
    on_imported: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=Callback::new(move |()| open.set(false))
            size=DialogSize::Lg
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.import_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <ImportPanel on_imported=on_imported />
            </DialogBody>
        </Dialog>
    }
}
