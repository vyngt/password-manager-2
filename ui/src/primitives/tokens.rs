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

impl Variant {
    pub fn btn_class(&self) -> &'static str {
        match self {
            Variant::Primary => "btn--primary",
            Variant::Secondary => "btn--secondary",
            Variant::Ghost => "btn--ghost",
            Variant::Danger => "btn--danger",
            Variant::Warning => "btn--warning",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Variant::Primary => "icon-btn--primary",
            Variant::Secondary => "icon-btn--secondary",
            Variant::Ghost => "icon-btn--ghost",
            Variant::Danger => "icon-btn--danger",
            Variant::Warning => "icon-btn--warning",
        }
    }
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

impl Size {
    pub fn btn_class(&self) -> &'static str {
        match self {
            Size::Sm => "btn--sm",
            Size::Md => "btn--md",
            Size::Lg => "btn--lg",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Size::Sm => "icon-btn--sm",
            Size::Md => "icon-btn--md",
            Size::Lg => "icon-btn--lg",
        }
    }

    pub fn input_root_class(&self) -> &'static str {
        match self {
            Size::Sm => "input-root--sm",
            Size::Md => "",
            Size::Lg => "input-root--lg",
        }
    }
}

/// Field status — communicates validation state visually.
/// Used by form components (Label, Input, HelperText) to mirror
/// the associated control's validation result.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Status {
    #[default]
    Default,
    Error,
    Success,
    Warning,
}

impl Status {
    pub fn label_class(&self) -> &'static str {
        match self {
            Status::Default => "",
            Status::Error => "label--error",
            Status::Success => "label--success",
            Status::Warning => "label--warning",
        }
    }

    pub fn input_root_class(&self) -> &'static str {
        match self {
            Status::Default => "",
            Status::Error => "input-root--error",
            Status::Success => "input-root--success",
            Status::Warning => "input-root--warning",
        }
    }
}

/// Border-radius shape.
///   square     = --radius-none (0px)
///   rounded-sm = --radius-sm   (4px)
///   rounded    = --radius      (6px) — default
///   pill       = --radius-full (9999px)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Shape {
    Square,
    RoundedSm,
    #[default]
    Rounded,
    Pill,
}

impl Shape {
    pub fn btn_class(&self) -> &'static str {
        match self {
            Shape::Square => "btn--square",
            Shape::RoundedSm => "btn--rounded-sm",
            Shape::Rounded => "btn--rounded",
            Shape::Pill => "btn--pill",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Shape::Square => "icon-btn--square",
            Shape::RoundedSm => "icon-btn--rounded-sm",
            Shape::Rounded => "icon-btn--rounded",
            Shape::Pill => "icon-btn--pill",
        }
    }
}
