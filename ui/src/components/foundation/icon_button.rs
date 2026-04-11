mod styles;

use crate::primitives::color::RgbColor;
use crate::primitives::tokens::{Size, Variant};
use leptos::prelude::*;

#[component]
pub fn IconButton(
    children: Children,
    #[prop(into)] color: Signal<RgbColor>,
    #[prop(attrs, default = Size::Md)] size: Size,
    #[prop(attrs, default = Variant::Ghost)] variant: Variant,
    #[prop(attrs, default = true)] auto_text_color: bool,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = styles::apply_base();
    let size_cls = styles::apply_size(size);
    let variant_cls = styles::apply_variant(variant);
    let cls = vec![base_cls, size_cls, variant_cls, "transition", class].join(" ");

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
        <button type="button" class=cls style=handle_style>
            <span class="sr-only">"IconButton"</span>
            <span class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 transform pointer-events-none">
                {children()}
            </span>
        </button>
    }
}
