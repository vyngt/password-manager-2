mod styles;

use crate::components::ripple::effect::{RippleColor, add_ripple};
use crate::primitives::color::RgbColor;
use crate::primitives::tokens::{Effect, Shape, Size, Variant};
use leptos::prelude::*;

#[component]
pub fn IconButton(
    children: Children,
    #[prop(into)] color: Signal<RgbColor>,
    #[prop(attrs, optional, default = None)] effect: Option<Effect>,
    #[prop(attrs, default = Size::Medium)] size: Size,
    #[prop(attrs, default = Variant::Filled)] variant: Variant,
    #[prop(attrs, default = Shape::Rounded)] shape: Shape,
    #[prop(attrs, default = true)] auto_text_color: bool,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = styles::apply_base();
    let effect_cls = styles::apply_effect(effect.clone());
    let size_cls = styles::apply_size(size);
    let shape_cls = styles::apply_shape(shape);
    let variant_cls = styles::apply_variant(variant);
    let cls = vec![
        base_cls,
        effect_cls,
        size_cls,
        shape_cls,
        variant_cls,
        "transition",
        class,
    ]
    .join(" ");

    let ripple_color = Memo::new(move |_| RippleColor {
        alpha: 0.2,
        color: color.get().calculate_white_black_text_color(None),
    });

    let handle_on_click = move |ev: web_sys::MouseEvent| {
        if let Some(eff) = effect {
            match eff {
                Effect::Ripple => add_ripple(ev, Some(ripple_color.get())),
            }
        }
    };

    let handle_style = move || {
        let c = color.get();
        let bg_color_var = format!("--background-color: {}", c.to_rgb_string());
        if auto_text_color {
            let text_color = c.calculate_white_black_text_color(None);
            let blend_color = c.calculate_white_black_text_color(Some(0.8));
            let text_color_var = format!("--text-color: {}", text_color.to_rgb_string());
            let text_color_80_var = format!("--text-color-80: {}", blend_color.to_rgb_string());
            format!("{};{};{};", bg_color_var, text_color_var, text_color_80_var)
        } else {
            format!("{};", bg_color_var)
        }
    };

    view! {
        <button type="button" class=cls on:click=move |ev| handle_on_click(ev) style=handle_style>
            <span class="sr-only">"IconButton"</span>
            <span class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 transform pointer-events-none">
                {children()}
            </span>
        </button>
    }
}
