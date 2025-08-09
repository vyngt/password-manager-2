pub mod base;
pub mod styles;

use leptos::{ev::Targeted, prelude::*};
use web_sys::{Event, HtmlInputElement};

use crate::types::color::RgbColor;

#[component]
pub fn Input(
    #[prop(into)] color: Signal<RgbColor>,
    #[prop(attrs, default = "text")] input_type: &'static str,
    #[prop(attrs)] id: &'static str,
    #[prop(attrs)] placeholder: &'static str,
    #[prop(attrs, default = "")] class: &'static str,
    #[prop(attrs, default = "")] label_class: &'static str,

    #[prop(into)] on_input_target: Callback<Targeted<Event, HtmlInputElement>>,
) -> impl IntoView {
    let handle_on_input = move |ev: Targeted<Event, HtmlInputElement>| on_input_target.run(ev);

    let handle_style = move || {
        let c = color.get();
        let color_var = format!("--color: rgb({}, {}, {})", c.r, c.g, c.b);
        format!("{};", color_var)
    };

    let cls = vec!["peer input", class].join(" ");

    let label_cls = vec![
        "input--label before:content[' '] after:content:[' ']",
        label_class,
    ]
    .join(" ");

    view! {
        <div class="relative">
            <input
                style=handle_style
                class=cls
                id=id
                placeholder=" "
                on:input:target=handle_on_input
                type=input_type
            />
            <label
                style=handle_style
                class=label_cls
                for=id
            >
                {placeholder}
            </label>
        </div>
    }
}
