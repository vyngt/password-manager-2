use crate::components::foundation::badge::Badge;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{BadgeShape, BadgeVariant, Status};
use crate::utils::text::text_or;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Free-form tag creation control.
///
/// The user types a value and presses Enter (or comma) to create a chip.
/// Each chip is a pill-shaped Badge with a remove button. The result is a
/// fully-controlled `Vec<String>`. Not a multi-select — TagInput creates new
/// values, it does not pick from a list.
///
/// Integrates with FormField via the `id` prop (applied to the text input).
#[component]
pub fn TagInput(
    #[prop(into)] value: Signal<Vec<String>>,
    #[prop(into)] on_change: Callback<Vec<String>>,
    #[prop(into, default = TextProp::from("Add tag…"))] placeholder: TextProp,
    #[prop(optional, default = None)] max_tags: Option<u32>,
    #[prop(optional)] status: Status,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "")] id: &'static str,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(into, default = TextProp::from("Remove {tag}"))] remove_label: TextProp,
    #[prop(into, default = TextProp::from("{tag} added"))] added_announce: TextProp,
    #[prop(into, default = TextProp::from("{tag} removed"))] removed_announce: TextProp,
    #[prop(into, default = TextProp::from("Maximum {n} tags reached"))]
    max_reached_announce: TextProp,
    #[prop(into, default = TextProp::from("Max {n} tags reached"))] max_reached_hint: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if matches!(max_tags, Some(0)) {
        web_sys::console::warn_1(
            &"TagInput: `max_tags=0` makes the input permanently unusable.".into(),
        );
    }

    let input_text = RwSignal::new(String::new());
    let announce = RwSignal::new(String::new());
    let root_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let max_reached = Signal::derive(move || match max_tags {
        Some(n) => value.get().len() >= n as usize,
        None => false,
    });

    // --- helpers -----------------------------------------------------------

    let add_tag = move |raw: String| {
        let tag = raw.trim().to_string();
        if tag.is_empty() {
            return;
        }
        let current = value.get_untracked();
        if current.iter().any(|t| t == &tag) {
            return;
        }
        if let Some(max) = max_tags {
            if current.len() >= max as usize {
                let msg = text_or(max_reached_announce, "Maximum {n} tags reached")
                    .replace("{n}", &max.to_string());
                announce.set(msg);
                return;
            }
        }
        let mut next = current;
        next.push(tag.clone());
        on_change.run(next);
        let msg = text_or(added_announce, "{tag} added").replace("{tag}", &tag);
        announce.set(msg);
    };

    let remove_tag = move |tag: String| {
        let next: Vec<String> = value
            .get_untracked()
            .into_iter()
            .filter(|t| t != &tag)
            .collect();
        on_change.run(next);
        let msg = text_or(removed_announce, "{tag} removed").replace("{tag}", &tag);
        announce.set(msg);
    };

    let focus_input = move || {
        if let Some(el) = input_ref.get_untracked() {
            let _ = el.focus();
        }
    };

    let focus_chip_at = move |target_idx: usize| {
        let Some(root) = root_ref.get_untracked() else {
            return;
        };
        let el: &web_sys::Element = root.unchecked_ref();
        let Ok(list) = el.query_selector_all(".tag-input__chip-remove") else {
            return;
        };
        let Some(node) = list.item(target_idx as u32) else {
            return;
        };
        if let Ok(btn) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = btn.focus();
        }
    };

    // --- event handlers ----------------------------------------------------

    let handle_input =
        move |ev: leptos::ev::Targeted<web_sys::Event, web_sys::HtmlInputElement>| {
            let text = ev.target().value();
            if let Some(last_comma) = text.rfind(',') {
                // Split: everything up to the last comma becomes tags; anything after
                // stays as the live text input value.
                let (before, after) = text.split_at(last_comma);
                for token in before.split(',') {
                    let t = token.trim();
                    if !t.is_empty() {
                        add_tag(t.to_string());
                    }
                }
                // Skip the comma itself (`after` starts with `,`)
                let remainder = after.get(1..).unwrap_or("").to_string();
                input_text.set(remainder);
                // Also write the value back to the DOM input so it visually clears.
                if let Some(el) = input_ref.get_untracked() {
                    el.set_value(&input_text.get_untracked());
                }
            } else {
                input_text.set(text);
            }
        };

    let handle_input_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            let text = input_text.get_untracked();
            add_tag(text);
            input_text.set(String::new());
            if let Some(el) = input_ref.get_untracked() {
                el.set_value("");
            }
        }
        "Backspace" if input_text.with_untracked(String::is_empty) => {
            let current = value.get_untracked();
            if let Some(last) = current.last().cloned() {
                remove_tag(last);
            }
        }
        "ArrowLeft" if input_text.with_untracked(String::is_empty) => {
            let len = value.get_untracked().len();
            if len > 0 {
                ev.prevent_default();
                focus_chip_at(len - 1);
            }
        }
        "Escape" => {
            input_text.set(String::new());
            if let Some(el) = input_ref.get_untracked() {
                el.set_value("");
            }
        }
        _ => {}
    };

    let handle_blur = move |ev: web_sys::FocusEvent| {
        // If focus is moving to another element inside our root (e.g. a chip
        // remove button), do not commit — user is navigating the component.
        if let Some(related) = ev.related_target() {
            if let Ok(node) = related.dyn_into::<web_sys::Node>() {
                if let Some(root) = root_ref.get_untracked() {
                    let root_node: &web_sys::Node = root.unchecked_ref();
                    if root_node.contains(Some(&node)) {
                        return;
                    }
                }
            }
        }
        let text = input_text.get_untracked();
        if !text.trim().is_empty() {
            add_tag(text);
            input_text.set(String::new());
            if let Some(el) = input_ref.get_untracked() {
                el.set_value("");
            }
        }
    };

    let handle_paste = move |ev: web_sys::ClipboardEvent| {
        let Some(data) = ev.clipboard_data() else {
            return;
        };
        let Ok(text) = data.get_data("text/plain") else {
            return;
        };
        if text.contains(',') || text.contains('\n') {
            ev.prevent_default();
            for token in text.split(|c| c == ',' || c == '\n') {
                let t = token.trim();
                if !t.is_empty() {
                    add_tag(t.to_string());
                }
            }
        }
    };

    let handle_root_click = move |ev: web_sys::MouseEvent| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(el) = target.dyn_into::<web_sys::Element>() else {
            return;
        };
        // Only grab focus when the user clicked the bare container — not a
        // chip, a chip remove button, or the input itself.
        if el.tag_name() == "DIV" || el.tag_name() == "UL" {
            focus_input();
        }
    };

    // --- rendering ---------------------------------------------------------

    let root_cls = Signal::derive(move || {
        let status_cls = match status {
            Status::Error => "tag-input--error",
            Status::Success => "tag-input--success",
            Status::Warning => "tag-input--warning",
            Status::Default => "",
        };
        [
            "tag-input",
            status_cls,
            if disabled { "tag-input--disabled" } else { "" },
            class,
        ]
        .join(" ")
    });

    let aria_described = (!aria_describedby.is_empty()).then_some(aria_describedby);

    view! {
        <div class=root_cls node_ref=root_ref on:click=handle_root_click>
            <ul class="tag-input__chips" role="list">
                <For
                    each=move || value.get()
                    key=|tag| tag.clone()
                    children=move |tag| {
                        let tag_for_aria = tag.clone();
                        let tag_for_click = tag.clone();
                        let tag_for_kbd = tag.clone();
                        let aria = text_or(remove_label, "Remove {tag}")
                            .replace("{tag}", &tag_for_aria);
                        let handle_chip_click = move |_: web_sys::MouseEvent| {
                            remove_tag(tag_for_click.clone());
                            focus_input();
                        };
                        let handle_chip_keydown = move |ev: web_sys::KeyboardEvent| {
                            let current = value.get_untracked();
                            let Some(my_idx) = current.iter().position(|t| t == &tag_for_kbd) else {
                                return;
                            };
                            match ev.key().as_str() {
                                "Delete" | "Backspace" => {
                                    ev.prevent_default();
                                    remove_tag(tag_for_kbd.clone());
                                    focus_input();
                                }
                                "ArrowRight" => {
                                    ev.prevent_default();
                                    let next = my_idx + 1;
                                    if next < current.len() {
                                        focus_chip_at(next);
                                    } else {
                                        focus_input();
                                    }
                                }
                                "ArrowLeft" => {
                                    ev.prevent_default();
                                    if my_idx > 0 {
                                        focus_chip_at(my_idx - 1);
                                    }
                                }
                                _ => {}
                            }
                        };

                        view! {
                            <li role="listitem" class="tag-input__chip-item">
                                <Badge variant=BadgeVariant::Default shape=BadgeShape::Pill>
                                    <span class="tag-input__chip-text" title=tag.clone()>
                                        {tag.clone()}
                                    </span>
                                    <button
                                        type="button"
                                        class="tag-input__chip-remove"
                                        aria-label=aria
                                        disabled=disabled
                                        on:click=handle_chip_click
                                        on:keydown=handle_chip_keydown
                                    >
                                        "×"
                                    </button>
                                </Badge>
                            </li>
                        }
                    }
                />
            </ul>

            <Show when=move || !max_reached.get()>
                <input
                    node_ref=input_ref
                    id=id
                    type="text"
                    class="tag-input__native"
                    prop:value=move || input_text.get()
                    placeholder=move || placeholder.get()
                    disabled=disabled
                    aria-describedby=aria_described
                    on:input:target=handle_input
                    on:keydown=handle_input_keydown
                    on:blur=handle_blur
                    on:paste=handle_paste
                />
            </Show>

            <Show when=move || max_reached.get()>
                <span class="tag-input__max-hint">
                    {move || {
                        max_tags
                            .map(|n| {
                                text_or(max_reached_hint, "Max {n} tags reached")
                                    .replace("{n}", &n.to_string())
                            })
                            .unwrap_or_default()
                    }}
                </span>
            </Show>

            <span class="tag-input__sr-announce" aria-live="polite" aria-atomic="true">
                {move || announce.get()}
            </span>
        </div>
    }
}
