use crate::primitives::tokens::{Size, Variant};

pub fn apply_base() -> &'static str {
    "icon-btn"
}

pub fn apply_variant(variant: Variant) -> &'static str {
    match variant {
        Variant::Primary => "btn-filled",
        Variant::Secondary => "btn-outlined",
        Variant::Ghost => "btn-text",
        Variant::Danger => "",
        Variant::Warning => "",
    }
}

pub fn apply_size(size: Size) -> &'static str {
    match size {
        Size::Sm => "text-sm",
        Size::Md => "text-base",
        Size::Lg => "text-lg",
    }
}
