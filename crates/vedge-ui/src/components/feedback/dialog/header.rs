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
                        <IconButton
                            aria_label=ctx.close_label
                            variant=Variant::Ghost
                            size=Size::Sm
                            class="dialog__close"
                            on:click=move |_: web_sys::MouseEvent| ctx.on_close.run(())
                        >
                            <Icon icon=i::FaXmarkSolid />
                        </IconButton>
                    }
                })}
        </div>
    }
}
