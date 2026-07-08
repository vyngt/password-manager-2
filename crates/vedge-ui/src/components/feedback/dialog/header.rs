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
                        // `on_click=` (a direct binding on IconButton's `<button>`),
                        // NOT spread `on:click=`: the Dialog renders inside a
                        // `<Portal>`, and Leptos's spread-onto-component forwarding
                        // does not attach a listener across the portal boundary — a
                        // spread `on:click` here silently never fires (the ✕ wouldn't
                        // close). Do not "simplify" this back to `on:click`.
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
