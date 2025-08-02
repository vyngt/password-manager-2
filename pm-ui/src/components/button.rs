pub mod base;
pub mod ripple;
pub mod styles;

use crate::constants::Color;
use leptos::prelude::*;

use self::ripple::add_ripple;

use self::base::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};

#[component]
pub fn Button(
    children: Children,
    #[prop(attrs, default = Color::Primary)] color: Color,
    #[prop(attrs, default = ButtonEffect::None)] effect: ButtonEffect,
    #[prop(attrs, default = ButtonSize::Medium)] size: ButtonSize,
    #[prop(attrs, default = ButtonVariant::Filled)] variant: ButtonVariant,
    #[prop(attrs, default = ButtonShape::Rounded)] shape: ButtonShape,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = styles::apply_base();
    let effect_cls = styles::apply_effect(&effect);
    let size_cls = styles::apply_size(size);
    let variant_cls = styles::apply_variant(variant, color);
    let shape_cls = styles::apply_shape(shape);
    let cls = vec![
        base_cls,
        effect_cls,
        size_cls,
        variant_cls,
        shape_cls,
        class,
    ]
    .join(" ");

    let handle_on_click = move |ev: web_sys::MouseEvent| match &effect {
        ButtonEffect::Ripple => add_ripple(ev),
        ButtonEffect::None => {}
    };

    view! {
        <button type="button" class=cls on:click=handle_on_click>
           {children()}
        </button>
    }
}
