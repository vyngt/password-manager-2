use super::dialog::DialogContext;
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
                    // A plain <button> with a DIRECT `on:click` (not an <IconButton>
                    // with a spread handler) so the close reliably fires from inside
                    // the portal + ChildrenFn nesting; `dialog__close` gives it a
                    // stacking context so nothing in the header can cover it. Styled
                    // with the icon-button classes so it looks identical.
                    view! {
                        <button
                            type="button"
                            class="icon-btn icon-btn--ghost icon-btn--sm dialog__close"
                            aria-label=move || ctx.close_label.get()
                            on:click=move |_: web_sys::MouseEvent| {
                                // TEMP DIAGNOSTIC — remove once the ✕ is confirmed.
                                web_sys::console::log_1(
                                    &"[Dialog] ✕ clicked → running on_close".into(),
                                );
                                ctx.on_close.run(());
                            }
                            on:pointerdown=move |_: web_sys::PointerEvent| {
                                web_sys::console::log_1(&"[Dialog] ✕ pointerdown".into());
                            }
                        >
                            <Icon icon=i::FaXmarkSolid />
                        </button>
                    }
                })}
        </div>
    }
}
