use super::variants::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};

pub fn apply_base() -> &'static str {
    return "btn";
}

pub fn apply_variant(variant: ButtonVariant) -> &'static str {
    match variant {
        ButtonVariant::Filled => "btn-filled",
        ButtonVariant::Outlined => "btn-outlined",
    }
}

pub fn apply_effect(effect: &ButtonEffect) -> &'static str {
    match effect {
        ButtonEffect::Ripple => "btn-ripple",
        ButtonEffect::None => "",
    }
}

pub fn apply_size(size: ButtonSize) -> &'static str {
    match size {
        ButtonSize::Small => "text-sm",
        ButtonSize::Medium => "text-base",
        ButtonSize::Large => "text-lg",
    }
}

pub fn apply_shape(shape: ButtonShape) -> &'static str {
    match shape {
        ButtonShape::Sharp => "rounded-none",
        ButtonShape::Rounded => "rounded",
        ButtonShape::Pill => "rounded-full",
    }
}
