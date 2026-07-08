use super::dialog::DialogContext;
use crate::components::foundation::icon_button::IconButton;
use crate::primitives::tokens::{Size, Variant};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn DialogHeader(children: Children) -> impl IntoView {
    let ctx = expect_context::<DialogContext>();

    view! {
        <div class="dialog__header">
            <div class="dialog__header-content">{children()}</div>
            {ctx
                .closeable
                .then(|| {
                    view! {
                        // `on_click=` (a direct handler binding on IconButton's own
                        // `<button>`) rather than spread `on:click=`: an explicit
                        // handler prop is the clean way to wire a component-rendered
                        // button's click. `ctx.on_close` is the Dialog's *deferred*
                        // close (see dialog.rs `request_close`), which is what makes
                        // the in-Portal ✕ actually tear the dialog down.
                        <IconButton
                            aria_label=ctx.close_label
                            variant=Variant::Ghost
                            size=Size::Sm
                            class="dialog__close"
                            on_click=ctx.on_close
                        >
                            <Icon icon=i::FaXmarkSolid />
                        </IconButton>
                    }
                })}
        </div>
    }
}
