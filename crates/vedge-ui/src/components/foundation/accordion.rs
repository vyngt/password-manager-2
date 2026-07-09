use crate::primitives::tokens::Size;
use crate::utils::id::id_with_prefix;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AccordionMode {
    #[default]
    Single,
    Multiple,
}

pub struct AccordionItem {
    pub id: String,
    pub trigger: Children,
    pub content: Children,
    pub disabled: bool,
}

#[component]
pub fn Accordion(
    items: Vec<AccordionItem>,
    #[prop(optional)] mode: AccordionMode,
    #[prop(optional)] default_open: Vec<String>,
    #[prop(into, default = None)] on_change: Option<Callback<Vec<String>>>,
    #[prop(optional)] trigger_size: Size,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    {
        let ids: Vec<&String> = items.iter().map(|i| &i.id).collect();
        let mut sorted: Vec<&String> = ids.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != ids.len() {
            web_sys::console::error_1(&"Accordion: duplicate item `id` detected.".into());
        }
        for id in &default_open {
            if !ids.iter().any(|x| *x == id) {
                web_sys::console::warn_1(
                    &format!("Accordion: default_open id `{id}` not found in items.").into(),
                );
            }
        }
        if matches!(mode, AccordionMode::Single) && default_open.len() > 1 {
            web_sys::console::warn_1(
                &"Accordion: default_open has multiple ids but mode=Single — only the first is used.".into(),
            );
        }
    }

    let initial_open: Vec<String> = match mode {
        AccordionMode::Single => default_open.into_iter().take(1).collect(),
        AccordionMode::Multiple => default_open,
    };

    let open_ids = RwSignal::new(initial_open.clone());
    let instance = StoredValue::new(id_with_prefix("accordion"));

    let panel_refs: StoredValue<HashMap<String, NodeRef<leptos::html::Div>>> =
        StoredValue::new(HashMap::new());
    let trigger_refs: StoredValue<Vec<NodeRef<leptos::html::Button>>> =
        StoredValue::new(Vec::new());
    let anim_versions: StoredValue<HashMap<String, Arc<AtomicU32>>> =
        StoredValue::new(HashMap::new());

    let total = items.len();

    {
        let initial_open = initial_open.clone();
        Effect::new(move |_| {
            for id in &initial_open {
                if let Some(panel_ref) = panel_refs.with_value(|m| m.get(id).copied()) {
                    if let Some(el) = panel_ref.get_untracked() {
                        let html: &web_sys::HtmlElement = el.unchecked_ref();
                        let _ = html.style().set_property("height", "auto");
                        let _ = html.set_attribute("data-state", "open");
                    }
                }
            }
        });
    }

    let toggle = move |id: String| {
        let prev_open: Vec<String> = open_ids.get_untracked();
        let was_open = prev_open.iter().any(|x| x == &id);

        let next_open: Vec<String> = match mode {
            AccordionMode::Single => {
                if was_open {
                    vec![]
                } else {
                    vec![id.clone()]
                }
            }
            AccordionMode::Multiple => {
                if was_open {
                    prev_open.iter().filter(|x| *x != &id).cloned().collect()
                } else {
                    let mut v = prev_open.clone();
                    v.push(id.clone());
                    v
                }
            }
        };

        for item_id in prev_open.iter() {
            if !next_open.contains(item_id) {
                animate_panel(item_id, false, panel_refs, anim_versions);
            }
        }
        for item_id in next_open.iter() {
            if !prev_open.contains(item_id) {
                animate_panel(item_id, true, panel_refs, anim_versions);
            }
        }

        open_ids.set(next_open.clone());
        if let Some(cb) = on_change {
            cb.run(next_open);
        }
    };

    let items_views = items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let item_id = item.id.clone();
            let trigger_slot = item.trigger;
            let content_slot = item.content;
            let disabled = item.disabled;

            let panel_ref: NodeRef<leptos::html::Div> = NodeRef::new();
            let trigger_ref: NodeRef<leptos::html::Button> = NodeRef::new();
            let version = Arc::new(AtomicU32::new(0));

            panel_refs.update_value(|m| {
                m.insert(item_id.clone(), panel_ref);
            });
            trigger_refs.update_value(|v| v.push(trigger_ref));
            anim_versions.update_value(|m| {
                m.insert(item_id.clone(), version);
            });

            let trigger_html_id = format!("{}-trigger-{}", instance.get_value(), &item_id);
            let panel_html_id = format!("{}-panel-{}", instance.get_value(), &item_id);

            let is_open = {
                let id = item_id.clone();
                Signal::derive(move || open_ids.with(|v| v.iter().any(|x| x == &id)))
            };

            let trigger_cls =
                ["accordion__trigger", trigger_size.accordion_trigger_class()].join(" ");

            let toggle_click = toggle.clone();
            let click_id = item_id.clone();
            let handle_click = move |_: web_sys::MouseEvent| {
                if disabled {
                    return;
                }
                toggle_click(click_id.clone());
            };

            let handle_keydown = move |ev: web_sys::KeyboardEvent| {
                let target_idx = match ev.key().as_str() {
                    "ArrowDown" => (idx + 1) % total.max(1),
                    "ArrowUp" => {
                        if idx == 0 {
                            total.saturating_sub(1)
                        } else {
                            idx - 1
                        }
                    }
                    "Home" => 0,
                    "End" => total.saturating_sub(1),
                    _ => return,
                };
                ev.prevent_default();
                trigger_refs.with_value(|v| {
                    if let Some(r) = v.get(target_idx) {
                        if let Some(el) = r.get_untracked() {
                            let _ = el.focus();
                        }
                    }
                });
            };

            let trigger_id_attr = trigger_html_id.clone();
            let panel_id_attr = panel_html_id.clone();
            let trigger_id_labelledby = trigger_html_id.clone();

            view! {
                <div class="accordion__item">
                    <button
                        node_ref=trigger_ref
                        type="button"
                        id=trigger_id_attr
                        class=trigger_cls
                        disabled=disabled
                        aria-expanded=move || if is_open.get() { "true" } else { "false" }
                        aria-controls=panel_html_id
                        on:click=handle_click
                        on:keydown=handle_keydown
                    >
                        <span class="accordion__label">{trigger_slot()}</span>
                        <span class="accordion__chevron">
                            <Icon icon=i::FaChevronRightSolid />
                        </span>
                    </button>
                    <div
                        node_ref=panel_ref
                        class="accordion__panel"
                        id=panel_id_attr
                        role="region"
                        aria-labelledby=trigger_id_labelledby
                    >
                        <div class="accordion__panel-inner">{content_slot()}</div>
                    </div>
                </div>
            }
        })
        .collect_view();

    let root_cls = ["accordion", class].join(" ");

    view! { <div class=root_cls>{items_views}</div> }
}

fn animate_panel(
    id: &str,
    opening: bool,
    panel_refs: StoredValue<HashMap<String, NodeRef<leptos::html::Div>>>,
    anim_versions: StoredValue<HashMap<String, Arc<AtomicU32>>>,
) {
    let Some(panel_ref) = panel_refs.with_value(|m| m.get(id).copied()) else {
        return;
    };
    let Some(div) = panel_ref.get_untracked() else {
        return;
    };
    let el: web_sys::HtmlElement = div.unchecked_into();

    let Some(version) = anim_versions.with_value(|m| m.get(id).cloned()) else {
        return;
    };
    let ticket = version.fetch_add(1, Ordering::Relaxed) + 1;

    if opening {
        let _ = el.style().set_property("height", "0px");
        let _ = el.set_attribute("data-state", "opening");
        // Force layout so the starting height is committed before the rAF assignment.
        let _ = el.offset_height();

        let el_raf = el.clone();
        let ver_raf = version.clone();
        request_animation_frame(move || {
            if ver_raf.load(Ordering::Relaxed) != ticket {
                return;
            }
            let target = el_raf.scroll_height();
            let _ = el_raf
                .style()
                .set_property("height", &format!("{}px", target));
        });

        let el_done = el.clone();
        let ver_done = version.clone();
        set_timeout(
            move || {
                if ver_done.load(Ordering::Relaxed) != ticket {
                    return;
                }
                let _ = el_done.style().set_property("height", "auto");
                let _ = el_done.set_attribute("data-state", "open");
            },
            Duration::from_millis(210),
        );
    } else {
        let current = el.scroll_height();
        let _ = el.style().set_property("height", &format!("{}px", current));
        let _ = el.set_attribute("data-state", "closing");
        let _ = el.offset_height();

        let el_raf = el.clone();
        let ver_raf = version.clone();
        request_animation_frame(move || {
            if ver_raf.load(Ordering::Relaxed) != ticket {
                return;
            }
            let _ = el_raf.style().set_property("height", "0px");
        });

        let el_done = el.clone();
        let ver_done = version.clone();
        set_timeout(
            move || {
                if ver_done.load(Ordering::Relaxed) != ticket {
                    return;
                }
                let _ = el_done.set_attribute("data-state", "closed");
            },
            Duration::from_millis(160),
        );
    }
}
