use crate::components::feedback::popover::{Popover, PopoverPlacement};
use crate::components::foundation::kbd::Kbd;
use crate::components::foundation::separator::Separator;
use crate::primitives::text_prop::TextProp;
use icondata as i;
use icondata_core::Icon as IconData;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use wasm_bindgen::JsCast;

/// Visual variant of a menu item.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuItemVariant {
    #[default]
    Default,
    Danger,
    Disabled,
}

/// A single action in a menu.
///
/// Icons are passed as `icondata::Icon` descriptors (rendered via leptos_icons).
/// Shortcuts are plain strings, rendered automatically inside a `<Kbd>` element.
/// For fully custom item content, render your own menu rather than this
/// composite.
#[derive(Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub variant: MenuItemVariant,
    pub icon: Option<IconData>,
    pub shortcut: Option<String>,
    pub on_click: Option<Callback<()>>,
    pub href: Option<String>,
}

#[derive(Clone)]
pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

#[derive(Clone)]
pub struct MenuSection {
    pub label: Option<String>,
    pub items: Vec<MenuEntry>,
}

/// Anchored floating action list. See `docs/UI_Implementations/09-DropdownMenu.md`.
#[component]
pub fn DropdownMenu(
    trigger: Children,
    items: Vec<MenuSection>,
    #[prop(optional)] placement: PopoverPlacement,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_open: Option<Callback<()>>,
    #[prop(into, default = None)] on_close: Option<Callback<()>>,
    #[prop(into, default = TextProp::from("More options"))] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    {
        let mut seen: Vec<&String> = Vec::new();
        for section in &items {
            for entry in &section.items {
                if let MenuEntry::Item(it) = entry {
                    if seen.iter().any(|x| **x == it.id) {
                        web_sys::console::error_1(
                            &format!("DropdownMenu: duplicate item id `{}`.", it.id).into(),
                        );
                    }
                    seen.push(&it.id);
                    if it.on_click.is_some() && it.href.is_some() {
                        web_sys::console::warn_1(
                            &format!(
                                "DropdownMenu: item `{}` has both on_click and href — href wins, on_click still fires.",
                                it.id
                            )
                            .into(),
                        );
                    }
                }
            }
        }
    }

    let (open, set_open) = signal(false);
    let trigger_wrap_ref = NodeRef::<leptos::html::Div>::new();
    let state = MenuState::new(items);
    let items_sv = state.items;
    let focused_idx = state.focused_idx;
    let panel_ref = state.panel_ref;
    let focusable_count = state.focusable_count;

    let anchor = Signal::derive(move || {
        trigger_wrap_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    let do_close = Callback::new(move |_: ()| {
        set_open.set(false);
        if let Some(cb) = on_close {
            cb.run(());
        }
    });

    let open_menu = move |initial_idx: usize| {
        if disabled || focusable_count == 0 {
            return;
        }
        focused_idx.set(Some(initial_idx));
        set_open.set(true);
        if let Some(cb) = on_open {
            cb.run(());
        }
        set_timeout(
            move || focus_item_at(panel_ref, initial_idx),
            Duration::from_millis(20),
        );
    };

    let handle_trigger_click = move |_: web_sys::MouseEvent| {
        if disabled {
            return;
        }
        if open.get_untracked() {
            do_close.run(());
        } else {
            open_menu(0);
        }
    };

    let handle_trigger_keydown = move |ev: web_sys::KeyboardEvent| {
        if disabled {
            return;
        }
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                if !open.get_untracked() {
                    open_menu(0);
                }
            }
            "ArrowUp" => {
                ev.prevent_default();
                if !open.get_untracked() {
                    open_menu(focusable_count.saturating_sub(1));
                }
            }
            _ => {}
        }
    };

    let wrapper_cls = ["dropdown-menu-trigger", class].join(" ");

    view! {
        <div
            node_ref=trigger_wrap_ref
            class=wrapper_cls
            aria-haspopup="menu"
            aria-expanded=move || if open.get() { "true" } else { "false" }
            aria-disabled=move || if disabled { Some("true") } else { None }
            on:click=handle_trigger_click
            on:keydown=handle_trigger_keydown
        >
            {trigger()}
        </div>

        <Popover
            open=Signal::derive(move || open.get())
            on_close=do_close
            anchor=anchor
            placement=placement
        >
            <MenuPanelBody
                items=items_sv
                focused_idx=focused_idx
                focusable_count=focusable_count
                focusable_ids=state.focusable_ids
                panel_ref=panel_ref
                typeahead_buf=state.typeahead_buf
                typeahead_ver=state.typeahead_ver
                close_cb=do_close
                aria_label=aria_label
            />
        </Popover>
    }
}

/// Shared menu-panel state, built once at the parent component's setup.
/// Used by both DropdownMenu and ContextMenu.
#[derive(Clone, Copy)]
pub struct MenuState {
    pub items: StoredValue<Vec<MenuSection>>,
    pub focusable_ids: StoredValue<Vec<String>>,
    pub focusable_count: usize,
    pub focused_idx: RwSignal<Option<usize>>,
    pub panel_ref: NodeRef<leptos::html::Ul>,
    pub typeahead_buf: RwSignal<String>,
    pub typeahead_ver: StoredValue<Arc<AtomicU32>>,
}

impl MenuState {
    pub fn new(items: Vec<MenuSection>) -> Self {
        let focusable_ids: Vec<String> = items
            .iter()
            .flat_map(|s| {
                s.items.iter().filter_map(|e| match e {
                    MenuEntry::Item(it) if it.variant != MenuItemVariant::Disabled => {
                        Some(it.id.clone())
                    }
                    _ => None,
                })
            })
            .collect();
        let focusable_count = focusable_ids.len();
        Self {
            items: StoredValue::new(items),
            focusable_ids: StoredValue::new(focusable_ids),
            focusable_count,
            focused_idx: RwSignal::new(None),
            panel_ref: NodeRef::new(),
            typeahead_buf: RwSignal::new(String::new()),
            typeahead_ver: StoredValue::new(Arc::new(AtomicU32::new(0))),
        }
    }
}

fn focus_item_at(panel_ref: NodeRef<leptos::html::Ul>, idx: usize) {
    let Some(panel) = panel_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::Element = panel.unchecked_ref();
    let selector = format!("[data-menu-idx=\"{idx}\"]");
    if let Ok(Some(node)) = el.query_selector(&selector) {
        if let Ok(btn) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = btn.focus();
        }
    }
}

/// Shared menu-panel body — the `<ul role="menu">` with keyboard handling,
/// typeahead, roving tabindex, and item rendering. Used by both DropdownMenu
/// and ContextMenu inside their respective Popover.
#[component]
pub fn MenuPanelBody(
    items: StoredValue<Vec<MenuSection>>,
    focused_idx: RwSignal<Option<usize>>,
    focusable_count: usize,
    focusable_ids: StoredValue<Vec<String>>,
    panel_ref: NodeRef<leptos::html::Ul>,
    typeahead_buf: RwSignal<String>,
    typeahead_ver: StoredValue<Arc<AtomicU32>>,
    close_cb: Callback<()>,
    #[prop(into)] aria_label: TextProp,
) -> impl IntoView {
    let commit = move |id: String| {
        let cb_opt = items.with_value(|items| {
            items
                .iter()
                .flat_map(|s| s.items.iter())
                .find_map(|e| match e {
                    MenuEntry::Item(it) if it.id == id => it.on_click,
                    _ => None,
                })
        });
        if let Some(cb) = cb_opt {
            cb.run(());
        }
        close_cb.run(());
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if focusable_count == 0 {
            return;
        }
        let current = focused_idx.get_untracked().unwrap_or(0);

        let next_idx: Option<usize> = match key.as_str() {
            "ArrowDown" => Some((current + 1) % focusable_count),
            "ArrowUp" => Some(if current == 0 {
                focusable_count - 1
            } else {
                current - 1
            }),
            "Home" => Some(0),
            "End" => Some(focusable_count - 1),
            "Escape" => {
                ev.prevent_default();
                close_cb.run(());
                return;
            }
            "Tab" => {
                close_cb.run(());
                return;
            }
            "Enter" | " " | "Spacebar" => {
                ev.prevent_default();
                if let Some(id) = focusable_ids.with_value(|ids| ids.get(current).cloned()) {
                    commit(id);
                }
                return;
            }
            k if k.chars().count() == 1 => {
                let ch = k.chars().next().unwrap();
                if !ch.is_alphanumeric() {
                    return;
                }
                let mut buf = typeahead_buf.get_untracked();
                buf.push(ch.to_ascii_lowercase());
                typeahead_buf.set(buf.clone());

                let match_idx = items.with_value(|sections| {
                    let mut counter = 0usize;
                    let mut first: Option<usize> = None;
                    for section in sections {
                        for entry in &section.items {
                            if let MenuEntry::Item(it) = entry {
                                if it.variant == MenuItemVariant::Disabled {
                                    continue;
                                }
                                if it.label.to_ascii_lowercase().starts_with(&buf)
                                    && first.is_none()
                                {
                                    first = Some(counter);
                                }
                                counter += 1;
                            }
                        }
                    }
                    first
                });

                let ticket = typeahead_ver
                    .with_value(|ver| ver.fetch_add(1, Ordering::Relaxed))
                    + 1;
                let ver_clone = typeahead_ver.with_value(|ver| ver.clone());
                set_timeout(
                    move || {
                        if ver_clone.load(Ordering::Relaxed) == ticket {
                            typeahead_buf.set(String::new());
                        }
                    },
                    Duration::from_millis(500),
                );

                match_idx
            }
            _ => return,
        };

        if let Some(idx) = next_idx {
            ev.prevent_default();
            focused_idx.set(Some(idx));
            focus_item_at(panel_ref, idx);
        }
    };

    view! {
        <ul
            node_ref=panel_ref
            class="menu-panel"
            role="menu"
            aria-label=move || aria_label.get()
            on:keydown=handle_keydown
        >
            {move || render_sections(items, focused_idx, commit)}
        </ul>
    }
}

fn render_sections(
    items_sv: StoredValue<Vec<MenuSection>>,
    focused_idx: RwSignal<Option<usize>>,
    commit: impl Fn(String) + Copy + Send + Sync + 'static,
) -> AnyView {
    items_sv.with_value(|sections| {
        let mut nodes: Vec<AnyView> = Vec::new();
        let mut focusable_counter = 0usize;

        for section in sections {
            if let Some(label) = &section.label {
                let label_text = label.clone();
                nodes.push(
                    view! {
                        <li class="menu-group-label" role="presentation">{label_text}</li>
                    }
                    .into_any(),
                );
            }
            for entry in &section.items {
                match entry {
                    MenuEntry::Separator => {
                        nodes.push(
                            view! {
                                <li role="separator">
                                    <Separator />
                                </li>
                            }
                            .into_any(),
                        );
                    }
                    MenuEntry::Item(item) => {
                        let is_focusable = item.variant != MenuItemVariant::Disabled;
                        let this_idx = focusable_counter;
                        if is_focusable {
                            focusable_counter += 1;
                        }
                        nodes.push(render_menu_item(
                            item.clone(),
                            this_idx,
                            is_focusable,
                            focused_idx,
                            commit,
                        ));
                    }
                }
            }
        }
        nodes.into_any()
    })
}

fn render_menu_item(
    item: MenuItem,
    focusable_idx: usize,
    is_focusable: bool,
    focused_idx: RwSignal<Option<usize>>,
    commit: impl Fn(String) + Copy + Send + Sync + 'static,
) -> AnyView {
    let variant_cls = match item.variant {
        MenuItemVariant::Default => "",
        MenuItemVariant::Danger => "menu-item--danger",
        MenuItemVariant::Disabled => "menu-item--disabled",
    };
    let item_cls = ["menu-item", variant_cls].join(" ");

    let id_for_click = item.id.clone();
    let handle_click = move |_: web_sys::MouseEvent| {
        if !is_focusable {
            return;
        }
        commit(id_for_click.clone());
    };

    let is_active = Signal::derive(move || {
        is_focusable && focused_idx.get() == Some(focusable_idx)
    });
    let tabindex = move || if is_active.get() { "0" } else { "-1" };
    let aria_disabled = if !is_focusable { Some("true") } else { None };
    let data_idx_attr = if is_focusable {
        Some(focusable_idx.to_string())
    } else {
        None
    };

    let icon_view = item.icon.map(|ic| {
        view! {
            <span class="menu-item__icon" aria-hidden="true">
                <Icon icon=ic />
            </span>
        }
    });

    let shortcut_view = item.shortcut.clone().map(|text| view! {
        <span class="menu-item__hint">
            <Kbd>{text}</Kbd>
        </span>
    });

    let label_text = item.label.clone();

    if let Some(href) = item.href.clone() {
        view! {
            <li>
                <a
                    class=item_cls
                    href=href
                    role="menuitem"
                    tabindex=tabindex
                    aria-disabled=aria_disabled
                    data-menu-idx=data_idx_attr
                    on:click=handle_click
                >
                    {icon_view}
                    <span class="menu-item__label">{label_text}</span>
                    {shortcut_view}
                </a>
            </li>
        }
        .into_any()
    } else {
        view! {
            <li>
                <button
                    type="button"
                    class=item_cls
                    role="menuitem"
                    tabindex=tabindex
                    aria-disabled=aria_disabled
                    data-menu-idx=data_idx_attr
                    on:click=handle_click
                >
                    {icon_view}
                    <span class="menu-item__label">{label_text}</span>
                    {shortcut_view}
                </button>
            </li>
        }
        .into_any()
    }
}

// Reference icondata here so the unused-import warning on `i` stays silent in
// modules that include this file via re-exports (`icondata` is imported for
// future extension / example completeness).
#[allow(dead_code)]
fn _icondata_hint() -> IconData {
    i::FaEllipsisSolid
}
