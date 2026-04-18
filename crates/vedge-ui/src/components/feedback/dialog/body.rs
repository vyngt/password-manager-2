use super::dialog::DialogContext;
use crate::utils::id::id_with_prefix;
use leptos::prelude::*;

#[component]
pub fn DialogBody(children: Children) -> impl IntoView {
    let ctx = expect_context::<DialogContext>();
    let id = id_with_prefix("dialog-body");

    let id_for_ctx = id.clone();
    Effect::new(move |_| {
        ctx.set_body_id.set(Some(id_for_ctx.clone()));
    });
    on_cleanup(move || ctx.set_body_id.set(None));

    view! {
        <div id=id class="dialog__body">
            {children()}
        </div>
    }
}
