pub mod base;
pub mod ripple;
pub mod styles;

use crate::types::color::RgbColor;
use leptos::prelude::*;

use self::ripple::add_ripple;

use self::base::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};

#[component]
pub fn Button(
    children: Children,
    #[prop(into)] color: Signal<RgbColor>,
    #[prop(attrs, default = ButtonEffect::None)] effect: ButtonEffect,
    #[prop(attrs, default = ButtonSize::Medium)] size: ButtonSize,
    #[prop(attrs, default = ButtonVariant::Filled)] variant: ButtonVariant,
    #[prop(attrs, default = ButtonShape::Rounded)] shape: ButtonShape,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = styles::apply_base();
    let effect_cls = styles::apply_effect(&effect);
    let size_cls = styles::apply_size(size);
    let shape_cls = styles::apply_shape(shape);
    let variant_cls = styles::apply_variant(variant);
    let cls = vec![
        base_cls,
        effect_cls,
        size_cls,
        shape_cls,
        variant_cls,
        class,
        "transition",
    ]
    .join(" ");

    let handle_on_click = move |ev: web_sys::MouseEvent| match &effect {
        ButtonEffect::Ripple => add_ripple(ev),
        ButtonEffect::None => {}
    };

    view! {
        <button
            type="button"
            class=cls
            on:click=handle_on_click
            style=move || {
                let c = color.get();
                let text_color = c.calculate_white_black_text_color(None);
                let blend_color = c.calculate_white_black_text_color(Some(0.8));

                let bg_color_var = format!("--background-color: rgb({}, {}, {})", c.r, c.g, c.b);
                let text_color_var = format!(
                    "--text-color: rgb({}, {}, {})",
                    text_color.r, text_color.g, text_color.b
                );
                let text_color_80_var = format!(
                    "--text-color-80: rgb({}, {}, {})",
                    blend_color.r, blend_color.g, blend_color.b
                );
                format!("{};{};{};", bg_color_var, text_color_var, text_color_80_var)
            }
        >
            {children()}
        </button>
    }
}
