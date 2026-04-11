mod styles;

use crate::primitives::color::RgbColor;
use crate::primitives::tokens::{Size, Variant};
use leptos::prelude::*;

#[component]
pub fn Button(
    children: Children,
    #[prop(into, optional, default = Signal::derive(|| RgbColor::new(0, 0, 0)))] color: Signal<
        RgbColor,
    >,
    #[prop(attrs, default = Size::Md)] size: Size,
    #[prop(attrs, default = Variant::Secondary)] variant: Variant,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = styles::apply_base();
    let size_cls = styles::apply_size(size);
    let variant_cls = styles::apply_variant(variant);
    let cls = vec![base_cls, size_cls, variant_cls, "transition", class].join(" ");

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
        <button type="button" class=cls style=handle_style>
            {children()}
        </button>
    }
}
