use super::dialog::DialogContext;
use crate::utils::id::id_with_prefix;
use leptos::prelude::*;

#[component]
pub fn DialogBody(
    /// Drop the body's padding so list/edge-to-edge content runs to the border.
    /// Default `false` is the safe (padded) case — forgetting it can't produce a
    /// broken-looking dialog.
    #[prop(optional)]
    flush: bool,
    children: Children,
) -> impl IntoView {
    let ctx = expect_context::<DialogContext>();
    let id = id_with_prefix("dialog-body");

    let id_for_ctx = id.clone();
    Effect::new(move |_| {
        ctx.set_body_id.set(Some(id_for_ctx.clone()));
    });
    on_cleanup(move || ctx.set_body_id.set(None));

    let class = [
        "dialog__body",
        if flush { "dialog__body--flush" } else { "" },
    ]
    .join(" ");

    view! {
        <div id=id class=class>
            {children()}
        </div>
    }
}
