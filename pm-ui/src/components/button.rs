use crate::constants::Color;
use leptos::prelude::*;

#[component]
pub fn Button(
    children: Children,
    #[prop(attrs, default = Color::Primary)] color: Color,
) -> impl IntoView {
    let cls = match color {
        Color::Primary => {
            view! { <{..} class="btn bg-primary hover:bg-primary/80" /> }
        }
        Color::Secondary => view! { <{..} class="btn bg-secondary hover:bg-secondary/80" /> },
        Color::Success => view! { <{..} class="btn bg-success hover:bg-success/80" /> },
        Color::Danger => view! { <{..} class="btn bg-danger hover:bg-danger/80" /> },
        Color::Warning => view! { <{..} class="btn bg-warning hover:bg-warning/80" /> },
        _ => view! { <{..} class="btn bg-primary hover:bg-primary/80" /> },
    };

    view! {
        <button type="button" {..cls}>
           {children()}
        </button>
    }
}
