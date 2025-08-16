pub mod styles;
pub mod variants;

use crate::types::color::RgbColor;
use leptos::prelude::*;

use super::ripple::effect::{RippleColor, add_ripple};

use self::variants::{Effect, Shape, Size, Variant};

#[component]
pub fn IconButton(
    children: Children,
    #[prop(into)] color: Signal<RgbColor>,
    #[prop(attrs, default = Effect::None)] effect: Effect,
    #[prop(attrs, default = Size::Medium)] size: Size,
    #[prop(attrs, default = Variant::Filled)] variant: Variant,
    #[prop(attrs, default = Shape::Rounded)] shape: Shape,
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
        "transition",
        class,
    ]
    .join(" ");

    let ripple_color = Memo::new(move |_| RippleColor {
        alpha: 0.2,
        color: color.get().calculate_white_black_text_color(None),
    });

    let handle_on_click = move |ev: web_sys::MouseEvent| match &effect {
        Effect::Ripple => add_ripple(ev, Some(ripple_color.get())),
        Effect::None => {}
    };

    let handle_style = move || {
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
