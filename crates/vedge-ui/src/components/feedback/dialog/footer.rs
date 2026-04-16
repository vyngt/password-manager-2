use leptos::prelude::*;

#[component]
pub fn DialogFooter(children: Children) -> impl IntoView {
    view! { <div class="dialog__footer">{children()}</div> }
}
