use super::base::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};
use crate::constants::Color;

pub fn apply_base() -> &'static str {
    return "btn";
}

pub fn apply_variant(variant: ButtonVariant, color: Color) -> &'static str {
    match variant {
        ButtonVariant::Filled => match color {
            Color::Primary => "bg-primary hover:bg-primary/80",
            Color::Secondary => "bg-secondary hover:bg-secondary/80",
            Color::Success => "bg-success hover:bg-success/80",
            Color::Danger => "bg-danger hover:bg-danger/80",
            Color::Warning => "bg-warning hover:bg-warning/80",
            _ => "",
        },
        ButtonVariant::Outlined => match color {
            Color::Primary => "border border-2 border-primary hover:bg-primary/10 text-primary",
            Color::Secondary => {
                "border border-2 border-secondary hover:bg-secondary/10 text-secondary"
            }
            Color::Success => "border border-2 border-success hover:bg-success/10 text-success",
            Color::Danger => "border border-2 border-danger hover:bg-danger/10 text-danger",
            Color::Warning => "border border-2 border-warning hover:bg-warning/10 text-warning",
            _ => "",
        },
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
