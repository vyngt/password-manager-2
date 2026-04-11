/// Semantic variant — expresses intent, not visual style.
/// The token system maps each variant to colors via CSS custom properties.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Variant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
    Warning,
}

/// Physical scale — controls height, font-size, padding.
/// Derived from the typography + spacing system:
///   sm = 28px = 16px line-height + 2×6px padding
///   md = 36px = 20px line-height + 2×8px padding
///   lg = 44px = 24px line-height + 2×10px padding
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}
