use crate::utils::id::id_with_prefix;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TabsVariant {
    #[default]
    Underline,
    Pill,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TabsActivation {
    #[default]
    Automatic,
    Manual,
}

pub struct Tab {
    pub id: String,
    pub label: Children,
    pub panel: ChildrenFn,
    pub disabled: bool,
}

#[component]
pub fn Tabs(
    tabs: Vec<Tab>,
    #[prop(into, default = None)] active: Option<Signal<String>>,
    #[prop(optional, default = None)] default_active: Option<String>,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    #[prop(optional)] variant: TabsVariant,
    #[prop(optional)] activation: TabsActivation,
    #[prop(optional, default = true)] lazy: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    {
        let mut seen: Vec<&String> = Vec::new();
        for t in &tabs {
            if seen.iter().any(|x| **x == t.id) {
                web_sys::console::error_1(&format!("Tabs: duplicate id `{}`.", t.id).into());
            }
            seen.push(&t.id);
        }
        if let Some(d) = &default_active {
            if !tabs.iter().any(|t| &t.id == d) {
                web_sys::console::warn_1(
                    &format!("Tabs: default_active `{d}` not in tabs list.").into(),
                );
            }
        }
        if active.is_some() && on_change.is_none() {
            web_sys::console::warn_1(
                &"Tabs: controlled `active` prop provided without `on_change` handler.".into(),
            );
        }
    }

    let initial_id = default_active
        .clone()
        .or_else(|| {
            tabs.iter()
                .find(|t| !t.disabled)
                .map(|t| t.id.clone())
                .or_else(|| tabs.first().map(|t| t.id.clone()))
        })
        .unwrap_or_default();

    let internal_active = RwSignal::new(initial_id);

    let effective_active = Signal::derive(move || match active {
        Some(s) => s.get(),
        None => internal_active.get(),
    });

    let instance = StoredValue::new(id_with_prefix("tabs"));
    let list_ref = NodeRef::<leptos::html::Div>::new();
    let indicator_left = RwSignal::new(0.0_f64);
    let indicator_width = RwSignal::new(0.0_f64);

    // Snapshot of tabs metadata (ids + disabled) for keyboard navigation.
    let tab_meta: Vec<(String, bool)> = tabs.iter().map(|t| (t.id.clone(), t.disabled)).collect();
    let tab_meta = StoredValue::new(tab_meta);

    let trigger_refs: StoredValue<Vec<NodeRef<leptos::html::Button>>> =
        StoredValue::new(Vec::with_capacity(tabs.len()));

    let commit_active = move |id: String| {
        if active.is_none() {
            internal_active.set(id.clone());
        }
        if let Some(cb) = on_change {
            cb.run(id);
        }
    };

    let focus_trigger = move |idx: usize| {
        if let Some(node) = trigger_refs
            .with_value(|refs| refs.get(idx).copied())
            .and_then(|r| r.get_untracked())
        {
            let _ = node.focus();
        }
    };

    let next_enabled_idx = move |from: usize, forward: bool, wrap: bool| -> Option<usize> {
        tab_meta.with_value(|meta| {
            let n = meta.len();
            if n == 0 {
                return None;
            }
            let mut idx = from;
            for _ in 0..n {
                idx = if forward {
                    if idx + 1 >= n {
                        if wrap {
                            0
                        } else {
                            return None;
                        }
                    } else {
                        idx + 1
                    }
                } else if idx == 0 {
                    if wrap {
                        n - 1
                    } else {
                        return None;
                    }
                } else {
                    idx - 1
                };
                if !meta[idx].1 {
                    return Some(idx);
                }
            }
            None
        })
    };

    // Indicator update — runs on mount and whenever the active tab changes.
    Effect::new(move |_| {
        let active_id = effective_active.get();
        request_animation_frame(move || {
            let idx = tab_meta.with_value(|m| m.iter().position(|(id, _)| id == &active_id));
            let Some(idx) = idx else {
                return;
            };
            let Some(node) = trigger_refs
                .with_value(|refs| refs.get(idx).copied())
                .and_then(|r| r.get_untracked())
            else {
                return;
            };
            let html: &web_sys::HtmlElement = node.unchecked_ref();
            indicator_left.set(html.offset_left() as f64);
            indicator_width.set(html.offset_width() as f64);
        });
    });

    // Window resize — reposition indicator. Closure is !Sync, so we store it
    // in a LocalStorage StoredValue and let its Drop run on owner dispose.
    let resize_cleanup: StoredValue<Option<Box<dyn FnOnce()>>, LocalStorage> =
        StoredValue::new_local(None);
    if let Some(window) = web_sys::window() {
        let reposition = move || {
            let active_id = effective_active.get_untracked();
            let idx = tab_meta.with_value(|m| m.iter().position(|(id, _)| id == &active_id));
            let Some(idx) = idx else {
                return;
            };
            let Some(node) = trigger_refs
                .with_value(|refs| refs.get(idx).copied())
                .and_then(|r| r.get_untracked())
            else {
                return;
            };
            let html: &web_sys::HtmlElement = node.unchecked_ref();
            indicator_left.set(html.offset_left() as f64);
            indicator_width.set(html.offset_width() as f64);
        };
        let closure =
            Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| reposition());
        let _ = window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
        let win = window.clone();
        let cleanup: Box<dyn FnOnce()> = Box::new(move || {
            let _ =
                win.remove_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            drop(closure);
        });
        resize_cleanup.set_value(Some(cleanup));
    }
    on_cleanup(move || {
        if let Some(f) = resize_cleanup.try_update_value(|v| v.take()).flatten() {
            f();
        }
    });

    let root_cls = [
        "tabs",
        match variant {
            TabsVariant::Underline => "",
            TabsVariant::Pill => "tabs--pill",
        },
        class,
    ]
    .join(" ");

    let instance_value = instance.get_value();

    // Build triggers + panels once, consuming the items vec.
    let mut triggers = Vec::with_capacity(tabs.len());
    let mut panels = Vec::with_capacity(tabs.len());

    for (idx, tab) in tabs.into_iter().enumerate() {
        let tab_id = tab.id.clone();
        let disabled = tab.disabled;
        let trigger_html_id = format!("{instance_value}-trigger-{tab_id}");
        let panel_html_id = format!("{instance_value}-panel-{tab_id}");

        let trigger_ref: NodeRef<leptos::html::Button> = NodeRef::new();
        trigger_refs.update_value(|v| v.push(trigger_ref));

        let is_active = {
            let id = tab_id.clone();
            Signal::derive(move || effective_active.get() == id)
        };

        // Trigger handlers
        let activate_id = tab_id.clone();
        let commit_a = commit_active;
        let handle_click = move |_: web_sys::MouseEvent| {
            if disabled {
                return;
            }
            commit_a(activate_id.clone());
        };

        let kbd_id = tab_id.clone();
        let handle_keydown = move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            let activate_on_move = matches!(activation, TabsActivation::Automatic);

            let target_idx = match key.as_str() {
                "ArrowRight" => next_enabled_idx(idx, true, true),
                "ArrowLeft" => next_enabled_idx(idx, false, true),
                "Home" => tab_meta.with_value(|m| m.iter().position(|(_, d)| !*d)),
                "End" => tab_meta.with_value(|m| {
                    m.iter()
                        .enumerate()
                        .rev()
                        .find(|(_, (_, d))| !*d)
                        .map(|(i, _)| i)
                }),
                "Enter" | " " | "Spacebar" => {
                    if matches!(activation, TabsActivation::Manual) && !disabled {
                        ev.prevent_default();
                        commit_a(kbd_id.clone());
                    }
                    return;
                }
                _ => return,
            };

            let Some(next_idx) = target_idx else {
                return;
            };
            ev.prevent_default();
            focus_trigger(next_idx);
            if activate_on_move {
                if let Some(id) = tab_meta.with_value(|m| m.get(next_idx).map(|(id, _)| id.clone()))
                {
                    commit_a(id);
                }
            }
        };

        let tabindex = Signal::derive({
            let id = tab_id.clone();
            move || {
                if effective_active.get() == id && !disabled {
                    "0"
                } else {
                    "-1"
                }
            }
        });

        let trigger_id_attr = trigger_html_id.clone();
        let panel_id_attr_for_trigger = panel_html_id.clone();

        triggers.push(view! {
            <button
                node_ref=trigger_ref
                type="button"
                id=trigger_id_attr
                class="tabs__trigger"
                role="tab"
                aria-selected=move || if is_active.get() { "true" } else { "false" }
                aria-controls=panel_id_attr_for_trigger
                aria-disabled=move || if disabled { Some("true") } else { None }
                tabindex=tabindex
                on:click=handle_click
                on:keydown=handle_keydown
            >
                {(tab.label)()}
            </button>
        });

        // Panel
        let panel_fn = tab.panel;
        let panel_fn_stored = StoredValue::new(panel_fn);
        // IDs need to be re-read each mount when wrapped in <Show>, so store
        // them in a StoredValue that the children closure can read freely.
        let trigger_id_sv = StoredValue::new(trigger_html_id.clone());
        let panel_id_sv = StoredValue::new(panel_html_id.clone());

        let panel_view = if lazy {
            // The entire <div> is mounted/unmounted, so there's no leftover
            // padded box adding space to the page when the tab is inactive.
            view! {
                <Show when=move || is_active.get()>
                    <div
                        role="tabpanel"
                        id=panel_id_sv.get_value()
                        aria-labelledby=trigger_id_sv.get_value()
                        class="tabs__panel"
                    >
                        {panel_fn_stored.with_value(|f| f())}
                    </div>
                </Show>
            }
            .into_any()
        } else {
            // Non-lazy: keep the div mounted so form state inside the panel
            // survives tab switches; the browser's default `[hidden]` styling
            // (display: none) ensures no space is reserved.
            view! {
                <div
                    role="tabpanel"
                    id=panel_id_sv.get_value()
                    aria-labelledby=trigger_id_sv.get_value()
                    class="tabs__panel"
                    hidden=move || !is_active.get()
                >
                    {panel_fn_stored.with_value(|f| f())}
                </div>
            }
            .into_any()
        };

        panels.push(panel_view);
    }

    let indicator_style = move || {
        format!(
            "transform: translateX({}px); width: {}px;",
            indicator_left.get(),
            indicator_width.get(),
        )
    };

    view! {
        <div class=root_cls>
            <div node_ref=list_ref class="tabs__list" role="tablist">
                {triggers}
                <div class="tabs__indicator" style=indicator_style aria-hidden="true" />
            </div>
            {panels}
        </div>
    }
}
