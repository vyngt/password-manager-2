use super::dialog::DialogContext;
use crate::utils::id::id_with_prefix;
use leptos::prelude::*;

#[component]
pub fn DialogTitle(children: Children) -> impl IntoView {
    let ctx = expect_context::<DialogContext>();
    let id = id_with_prefix("dialog-title");

    let id_for_ctx = id.clone();
    Effect::new(move |_| {
        ctx.set_title_id.set(Some(id_for_ctx.clone()));
    });
    on_cleanup(move || ctx.set_title_id.set(None));

    view! {
        <h2 id=id class="dialog__title">
            {children()}
        </h2>
    }
}
