/// Semantic variant — expresses intent, not visual style.
/// The token system maps each variant to colors via CSS custom properties.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Variant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
    Warning,
    /// Text link — transparent, no chrome, underline on hover. For inline
    /// actions styled as links (e.g. "use master password"). Buttons only;
    /// `IconButton` falls back to `Ghost`.
    Link,
}

impl Variant {
    pub fn btn_class(&self) -> &'static str {
        match self {
            Self::Primary => "btn--primary",
            Self::Secondary => "btn--secondary",
            Self::Ghost => "btn--ghost",
            Self::Danger => "btn--danger",
            Self::Warning => "btn--warning",
            Self::Link => "btn--link",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Self::Primary => "icon-btn--primary",
            Self::Secondary => "icon-btn--secondary",
            Self::Danger => "icon-btn--danger",
            Self::Warning => "icon-btn--warning",
            // Icon buttons have no link style; Link falls back to ghost.
            Self::Ghost | Self::Link => "icon-btn--ghost",
        }
    }
}

/// Physical scale — controls height, font-size, padding.
///
/// Derived from the typography + spacing system:
///   xs = 20px — compact icon actions in dense rows (`IconButton` only; other
///        components treat it as `Sm`)
///   sm = 28px = 16px line-height + 2×6px padding
///   md = 36px = 20px line-height + 2×8px padding
///   lg = 44px = 24px line-height + 2×10px padding
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Size {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub fn btn_class(&self) -> &'static str {
        match self {
            // No text-button `xs`; degrade to `sm` (xs is for icon actions).
            Self::Xs | Self::Sm => "btn--sm",
            Self::Md => "btn--md",
            Self::Lg => "btn--lg",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Self::Xs => "icon-btn--xs",
            Self::Sm => "icon-btn--sm",
            Self::Md => "icon-btn--md",
            Self::Lg => "icon-btn--lg",
        }
    }

    pub fn spinner_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "spinner--sm",
            Self::Md => "spinner--md",
            Self::Lg => "spinner--lg",
        }
    }

    pub fn input_root_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "input-root--sm",
            Self::Md => "",
            Self::Lg => "input-root--lg",
        }
    }

    pub fn number_input_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "number-input--sm",
            Self::Md => "",
            Self::Lg => "number-input--lg",
        }
    }

    pub fn segmented_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "segmented--sm",
            Self::Md | Self::Lg => "segmented--md",
        }
    }

    pub fn textarea_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "textarea--sm",
            Self::Md => "",
            Self::Lg => "textarea--lg",
        }
    }

    pub fn select_trigger_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "select-trigger--sm",
            Self::Md => "",
            Self::Lg => "select-trigger--lg",
        }
    }

    pub fn color_picker_trigger_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "color-picker-trigger--sm",
            Self::Md | Self::Lg => "",
        }
    }

    pub fn color_picker_trigger_only_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "color-picker-trigger-only--sm",
            Self::Md | Self::Lg => "",
        }
    }

    pub fn date_picker_trigger_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "datepicker-trigger--sm",
            Self::Md => "",
            Self::Lg => "datepicker-trigger--lg",
        }
    }

    pub fn progress_track_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "progress__track--sm",
            Self::Md => "progress__track--md",
            Self::Lg => "progress__track--lg",
        }
    }

    pub fn slider_root_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "slider-root--sm",
            Self::Md => "slider-root--md",
            Self::Lg => "slider-root--lg",
        }
    }

    pub fn qrcode_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm => "qrcode--sm",
            Self::Md => "qrcode--md",
            Self::Lg => "qrcode--lg",
        }
    }

    pub fn accordion_trigger_class(&self) -> &'static str {
        match self {
            Self::Xs | Self::Sm | Self::Md => "",
            Self::Lg => "accordion__trigger--lg",
        }
    }
}

/// Field status — communicates validation state visually.
/// Used by form components (Label, Input, `HelperText`) to mirror
/// the associated control's validation result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
            Self::Default => "",
            Self::Error => "label--error",
            Self::Success => "label--success",
            Self::Warning => "label--warning",
        }
    }

    pub fn input_root_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "input-root--error",
            Self::Success => "input-root--success",
            Self::Warning => "input-root--warning",
        }
    }

    pub fn helper_text_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "helper-text--error",
            Self::Success => "helper-text--success",
            Self::Warning => "helper-text--warning",
        }
    }

    pub fn number_input_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "number-input--error",
            Self::Success => "number-input--success",
            Self::Warning => "number-input--warning",
        }
    }

    pub fn textarea_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "textarea--error",
            Self::Success => "textarea--success",
            Self::Warning => "textarea--warning",
        }
    }

    pub fn select_trigger_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "select-trigger--error",
            Self::Success => "select-trigger--success",
            Self::Warning => "select-trigger--warning",
        }
    }

    pub fn date_picker_trigger_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Error => "datepicker-trigger--error",
            Self::Success => "datepicker-trigger--success",
            Self::Warning => "datepicker-trigger--warning",
        }
    }
}

/// Border-radius shape.
///   square     = --radius-none (0px)
///   rounded-sm = --radius-sm   (4px)
///   rounded    = --radius      (6px) — default
///   pill       = --radius-full (9999px)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
            Self::Square => "btn--square",
            Self::RoundedSm => "btn--rounded-sm",
            Self::Rounded => "btn--rounded",
            Self::Pill => "btn--pill",
        }
    }

    pub fn icon_btn_class(&self) -> &'static str {
        match self {
            Self::Square => "icon-btn--square",
            Self::RoundedSm => "icon-btn--rounded-sm",
            Self::Rounded => "icon-btn--rounded",
            Self::Pill => "icon-btn--pill",
        }
    }
}

/// Badge color variant — semantic, independent of interactive Variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Info,
    Success,
    Warning,
    Danger,
}

impl BadgeVariant {
    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Default => "badge--default",
            Self::Info => "badge--info",
            Self::Success => "badge--success",
            Self::Warning => "badge--warning",
            Self::Danger => "badge--danger",
        }
    }
}

/// Badge physical scale — only two sizes (no lg).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeSize {
    Sm,
    #[default]
    Md,
}

impl BadgeSize {
    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Sm => "badge--sm",
            Self::Md => "badge--md",
        }
    }
}

/// Badge shape — pill (default), square, or dot (indicator only).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeShape {
    #[default]
    Pill,
    Square,
    Dot,
}

impl BadgeShape {
    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Pill => "badge--pill",
            Self::Square => "badge--square",
            Self::Dot => "badge--dot",
        }
    }
}

/// Badge appearance — solid (muted bg) or outline (transparent + border).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeAppearance {
    #[default]
    Solid,
    Outline,
}

impl BadgeAppearance {
    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Solid => "badge--solid",
            Self::Outline => "badge--outline",
        }
    }
}

/// Binary-size control — Checkbox and Toggle only have two sizes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CheckboxSize {
    Sm,
    #[default]
    Md,
}

impl CheckboxSize {
    pub fn checkbox_class(&self) -> &'static str {
        match self {
            Self::Sm => "checkbox--sm",
            Self::Md => "checkbox--md",
        }
    }

    pub fn toggle_class(&self) -> &'static str {
        match self {
            Self::Sm => "toggle--sm",
            Self::Md => "toggle--md",
        }
    }
}

/// Separator orientation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn separator_class(&self) -> &'static str {
        match self {
            Self::Horizontal => "separator--horizontal",
            Self::Vertical => "separator--vertical",
        }
    }

    pub fn radio_group_class(&self) -> &'static str {
        match self {
            Self::Horizontal => "radio-group--horizontal",
            Self::Vertical => "radio-group--vertical",
        }
    }

    pub fn step_indicator_class(&self) -> &'static str {
        match self {
            Self::Horizontal => "step-indicator--horizontal",
            Self::Vertical => "step-indicator--vertical",
        }
    }
}

/// Floating element placement — preferred side relative to trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Placement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// Toast semantic variant — drives background, icon, and accent color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastVariant {
    #[default]
    Default,
    Success,
    Warning,
    Danger,
}

impl ToastVariant {
    pub fn toast_class(&self) -> &'static str {
        match self {
            Self::Default => "toast--default",
            Self::Success => "toast--success",
            Self::Warning => "toast--warning",
            Self::Danger => "toast--danger",
        }
    }
}

/// Horizontal text alignment for tabular data.
/// Start (default) = left in LTR, right in RTL. End = the inverse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    End,
}

impl Align {
    pub fn data_table_th_class(&self) -> &'static str {
        match self {
            Self::Start => "",
            Self::End => "data-table__th--end",
        }
    }

    pub fn data_table_td_class(&self) -> &'static str {
        match self {
            Self::Start => "",
            Self::End => "data-table__td--end",
        }
    }
}

/// Sort direction — used by sortable tables and data grids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_aria(&self) -> &'static str {
        match self {
            Self::Asc => "ascending",
            Self::Desc => "descending",
        }
    }
}

/// `ProgressBar` fill variant — determinate progress only (not for indeterminate loading,
/// which belongs to Spinner).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressVariant {
    #[default]
    Default,
    Success,
    Danger,
}

impl ProgressVariant {
    pub fn progress_fill_class(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Success => "progress__fill--success",
            Self::Danger => "progress__fill--danger",
        }
    }
}

/// Avatar diameter scale — five sizes (spec adds `xs` + `xl` beyond the base Size scale).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl AvatarSize {
    pub fn avatar_class(&self) -> &'static str {
        match self {
            Self::Xs => "avatar-root--xs",
            Self::Sm => "avatar-root--sm",
            Self::Md => "avatar-root--md",
            Self::Lg => "avatar-root--lg",
            Self::Xl => "avatar-root--xl",
        }
    }
}

/// Avatar presence indicator — communicates availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarStatus {
    Online,
    Away,
    Busy,
    Offline,
}

impl AvatarStatus {
    pub fn avatar_class(&self) -> &'static str {
        match self {
            Self::Online => "avatar-status--online",
            Self::Away => "avatar-status--away",
            Self::Busy => "avatar-status--busy",
            Self::Offline => "avatar-status--offline",
        }
    }

    pub fn aria_label(&self) -> &'static str {
        match self {
            Self::Online => "Status: online",
            Self::Away => "Status: away",
            Self::Busy => "Status: busy",
            Self::Offline => "Status: offline",
        }
    }
}

/// Dialog max-width scale. Separate from `Size` because `Full` (viewport-fill)
/// is a dialog-specific concept that does not fit the sm/md/lg typography scale.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DialogSize {
    Sm,
    #[default]
    Md,
    Lg,
    Full,
}

impl DialogSize {
    pub fn dialog_class(&self) -> &'static str {
        match self {
            Self::Sm => "dialog--sm",
            Self::Md => "dialog--md",
            Self::Lg => "dialog--lg",
            Self::Full => "dialog--full",
        }
    }
}
