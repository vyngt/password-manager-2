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
                            on:click=move |_| ctx.on_close.run(())
                        >
                            <Icon icon=i::FaXmarkSolid />
                        </IconButton>
                    }
                })}
        </div>
    }
}
