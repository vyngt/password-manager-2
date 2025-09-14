use super::variants::{Effect, Shape, Size, Variant};

pub fn apply_base() -> &'static str {
    return "btn";
}

pub fn apply_variant(variant: Variant) -> &'static str {
    match variant {
        Variant::Filled => "btn-filled",
        Variant::Outlined => "btn-outlined",
    }
}

pub fn apply_effect(effect: &Effect) -> &'static str {
    match effect {
        Effect::Ripple => "btn-ripple",
        Effect::None => "",
    }
}

pub fn apply_size(size: Size) -> &'static str {
    match size {
        Size::Small => "text-sm",
        Size::Medium => "text-base",
        Size::Large => "text-lg",
    }
}

pub fn apply_shape(shape: Shape) -> &'static str {
    match shape {
        Shape::Sharp => "rounded-none",
        Shape::Rounded => "rounded",
        Shape::Pill => "rounded-full",
    }
}
