use leptos::prelude::*;

#[derive(Clone, Copy)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[component]
pub fn Tooltip<F1, IV1, F2, IV2>(
    #[prop(optional)] position: Option<TooltipPosition>,

    #[prop(optional)] arrow: bool,

    #[prop(optional, default = "")] class: &'static str,

    trigger: F1,

    content: F2,
) -> impl IntoView
where
    F1: Fn() -> IV1 + Sync + Send + 'static,
    IV1: IntoView + 'static,
    F2: Fn() -> IV2 + Sync + Send + 'static,
    IV2: IntoView + 'static,
{
    let (hovered, set_hovered) = signal(false);
    let pos = position.unwrap_or(TooltipPosition::Top);

    // wrapper position (so với trigger)
    let wrapper_pos = match pos {
        TooltipPosition::Top => "bottom-full left-1/2 -translate-x-1/2 mb-2",
        TooltipPosition::Bottom => "top-full left-1/2 -translate-x-1/2 mt-2",
        TooltipPosition::Left => "right-full top-1/2 -translate-y-1/2 mr-2",
        TooltipPosition::Right => "left-full top-1/2 -translate-y-1/2 ml-2",
    };

    let pos_str = match pos {
        TooltipPosition::Top => "top",
        TooltipPosition::Bottom => "bottom",
        TooltipPosition::Left => "left",
        TooltipPosition::Right => "right",
    };

    let wrapper_cls = vec!["tooltip-wrapper", wrapper_pos].join(" ");
    let content_cls = vec!["tooltip", class].join(" ");
    let arrow_cls = vec!["tooltip-arrow"].join(" ");

    view! {
        <div
            class="relative inline-block"
            on:mouseenter=move |_| set_hovered.set(true)
            on:mouseleave=move |_| set_hovered.set(false)
            on:focus=move |_| set_hovered.set(true)
            on:blur=move |_| set_hovered.set(false)
        >
            {trigger().into_view()}

            <Show when=move || hovered.get()>
                <div class=wrapper_cls.clone() data-pos=pos_str>
                    <div class=content_cls.clone()>{content().into_view()}</div>

                    {arrow.then(|| view! { <div class=arrow_cls.clone()></div> })}
                </div>
            </Show>

        </div>
    }
}
