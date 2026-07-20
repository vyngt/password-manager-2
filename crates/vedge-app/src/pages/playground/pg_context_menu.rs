use icondata as i;
use leptos::prelude::*;
use vedge_ui::components::context_menu::ContextMenu;
use vedge_ui::components::dropdown_menu::{MenuEntry, MenuItem, MenuItemVariant, MenuSection};

use super::common::Section;

fn entry_items() -> Vec<MenuSection> {
    vec![MenuSection {
        label: None,
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "edit".into(),
                label: "Edit".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaPenSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "copy".into(),
                label: "Copy password".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaCopySolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "duplicate".into(),
                label: "Duplicate".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaCloneSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Separator,
            MenuEntry::Item(MenuItem {
                id: "delete".into(),
                label: "Delete".into(),
                variant: MenuItemVariant::Danger,
                icon: Some(i::FaTrashSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
        ],
    }]
}

#[component]
pub fn ContextMenuPage() -> impl IntoView {
    let (open_count, set_open_count) = signal(0_u32);
    let (close_count, set_close_count) = signal(0_u32);

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Context Menu"</h1>

            <Section title="Basic — right-click the box">
                <ContextMenu items=entry_items()>
                    <div class="w-full h-24 rounded-md border border-border bg-surface-1 flex items-center justify-center text-sm text-text-secondary">
                        "Right-click anywhere inside this box"
                    </div>
                </ContextMenu>
            </Section>

            <Section title="Disabled — falls back to browser native menu">
                <ContextMenu items=entry_items() disabled=true>
                    <div class="w-full h-24 rounded-md border border-border bg-surface-1 flex items-center justify-center text-sm text-text-secondary">
                        "Right-click here shows browser native context menu"
                    </div>
                </ContextMenu>
            </Section>

            <Section title="Near viewport edges — auto-flip">
                <div class="grid grid-cols-2 gap-4">
                    <ContextMenu items=entry_items()>
                        <div class="h-20 rounded-md border border-border bg-surface-1 flex items-center justify-center text-xs text-text-secondary">
                            "Top-left region"
                        </div>
                    </ContextMenu>
                    <ContextMenu items=entry_items()>
                        <div class="h-20 rounded-md border border-border bg-surface-1 flex items-center justify-center text-xs text-text-secondary">
                            "Top-right region"
                        </div>
                    </ContextMenu>
                </div>
            </Section>

            <Section title="With focusable children — focus restoration on close">
                <ContextMenu items=entry_items()>
                    <div class="p-4 rounded-md border border-border bg-surface-1 space-y-2">
                        <p class="text-sm text-text-secondary">
                            "Focus the input below, right-click anywhere, then press Escape. Focus should return to the input."
                        </p>
                        <input
                            type="text"
                            placeholder="Focus me, then right-click"
                            class="w-full rounded border border-border bg-background px-3 py-2 text-sm"
                        />
                    </div>
                </ContextMenu>
            </Section>

            <Section title="on_open / on_close counters">
                <ContextMenu
                    items=entry_items()
                    on_open=Callback::new(move |_: ()| set_open_count.update(|n| *n += 1))
                    on_close=Callback::new(move |_: ()| set_close_count.update(|n| *n += 1))
                >
                    <div class="w-full h-20 rounded-md border border-border bg-surface-1 flex items-center justify-center text-sm text-text-secondary">
                        "Tracked region"
                    </div>
                </ContextMenu>
                <div class="mt-2 text-xs text-text-secondary">
                    "Opens: "
                    <code class="text-text-primary">{move || open_count.get().to_string()}</code>
                    " · Closes: "
                    <code class="text-text-primary">{move || close_count.get().to_string()}</code>
                </div>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Context menu is pointer-only. Keyboard users access the same actions via a Dropdown Menu on each row. "
                "Inside the opened menu: Arrow Up/Down, Home/End, typeahead, Enter to activate, Escape/Tab/click-outside to close."
            </div>
        </div>
    }
}
