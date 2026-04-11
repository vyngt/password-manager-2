use crate::primitives::tokens::Status;
use leptos::prelude::*;

#[component]
pub fn Label(
    children: Children,
    html_for: &'static str,
    #[prop(optional)] required: bool,
    #[prop(optional)] optional: bool,
    #[prop(optional)] status: Status,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let cls = [
        "label",
        status.label_class(),
        if disabled { "label--disabled" } else { "" },
        class,
    ]
    .join(" ");

    view! {
        <label class=cls for=html_for>
            {children()}
            {required
                .then(|| {
                    view! {
                        <span class="label__required" aria-hidden="true">
                            "*"
                        </span>
                    }
                })}
            {(optional && !required)
                .then(|| view! { <span class="label__optional">"(optional)"</span> })}
        </label>
    }
}
