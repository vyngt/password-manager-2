pub mod base;
pub mod styles;

use leptos::{ev::Targeted, prelude::*};
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn Input(
    #[prop(attrs, default = "text")] input_type: &'static str,
    #[prop(attrs)] id: &'static str,
    #[prop(attrs)] placeholder: &'static str,
    #[prop(into)] on_input_target: Callback<Targeted<Event, HtmlInputElement>>,
) -> impl IntoView {
    let handle_on_input = move |ev: Targeted<Event, HtmlInputElement>| on_input_target.run(ev);

    view! {
        <input id=id placeholder=placeholder on:input:target=handle_on_input type=input_type />
        <label for=id />
    }
}
