# UI Package Reimplementation Plan

## Problem Statement

The current `ui/` implementation follows a **Material Design** pattern (Filled/Outlined/Text variants, Ripple effects, Shape props, `color: Signal<RgbColor>` injection). The design spec defines a **semantic token-driven** system where:

- Components consume CSS custom properties — never raw color values
- Variants express intent (primary/secondary/ghost/danger/warning) — not visual style
- The token system decides colors; components decide behavior; consumers decide intent
- No color prop exists (explicitly forbidden)

Every primitive, both Tier 1 components, and all CSS files need to be rewritten.

---

## Gap Analysis

### Primitives (`primitives/tokens.rs`)

| Aspect | Current | Spec |
|--------|---------|------|
| Variant | `Filled, Outlined, Text` | `Primary, Secondary, Ghost, Danger, Warning` |
| Shape | `Sharp, Rounded, Pill` | Not a concept — always `--radius` (6px) |
| Effect | `Ripple` | Not in spec — remove entirely |
| Size | `Small, Medium, Large` | `Sm, Md, Lg` (same concept, different values) |

### Primitives (`primitives/color.rs`)

| Aspect | Current | Spec |
|--------|---------|------|
| RgbColor struct | Used for dynamic color injection | **Remove** — components read CSS tokens, not RGB values |
| calculate_white_black_text_color | Computes contrast at runtime | **Remove** — token system provides `--color-*-foreground` |

### Button (`components/foundation/button.rs`)

| Aspect | Current | Spec |
|--------|---------|------|
| color prop | `Signal<RgbColor>` | **Forbidden** — no color prop |
| effect prop | `Option<Effect>` (ripple) | **Remove** — no ripple |
| shape prop | `Shape` | **Remove** — always 6px radius |
| variant | Filled/Outlined/Text | primary/secondary/ghost/danger/warning (default: **secondary**) |
| size | Small/Medium/Large | sm (28px) / md (36px) / lg (44px) (default: **md**) |
| disabled | Missing | Required — HTML attribute, opacity 0.4 |
| loading | Missing | Required — spinner replaces leading icon, aria-busy |
| leadingIcon | Missing | Optional icon slot (16×16) |
| trailingIcon | Missing | Optional icon slot (not on danger) |
| fullWidth | Missing | Optional boolean |
| type | Missing | button/submit/reset (default: button) |
| onClick | Missing (only ripple handler) | EventHandler, suppressed when disabled/loading |
| aria-label | Missing | Required when ambiguous |
| States | None | default, hover, focus, active, disabled, loading |
| CSS approach | Inline CSS vars from RgbColor | Semantic token consumption per variant |

### IconButton (`components/foundation/icon_button.rs`)

| Aspect | Current | Spec |
|--------|---------|------|
| color prop | `Signal<RgbColor>` | **Forbidden** |
| auto_text_color | bool | **Remove** — token system handles |
| children | Used for icon content | Replace with `icon` slot |
| aria-label | Missing | **Required** (never optional) |
| tooltip | Missing | Required (title attribute fallback) |
| Default variant | Filled | **ghost** |
| Geometry | Not enforced | Always square (width = height) |
| disabled/loading | Missing | Required |
| States | None | All 6 required |

### CSS Files

| File | Current | Spec |
|------|---------|------|
| button.css | `--background-color` from inline style, filled/outlined variants | Semantic tokens per variant (5), all states (hover/focus/active/disabled/loading) |
| icon-button.css | Same approach as button | Same token mapping as button, square geometry |
| ripple.css | Ripple animation | **Remove** |
| tokens.css | **Missing** | Must exist — master design token file |

---

## Implementation Plan

### Phase 1: Foundation — Tokens & Primitives

#### 1.1 Create `tokens.css`

Create `ui/src/styles/tokens.css` — the master design token file per the spec.

Contains all `@theme inline` tokens and `:root` fallback values:
- Primitive color layer (never used in components)
- Semantic colors: surfaces, primary, danger, warning, success, text hierarchy, borders
- Typography: font families, sizes with line-heights, weights, letter-spacing
- Motion: durations (instant/micro/fast/base/slow), easings (out/in/in-out/spring)
- Component sizing: heights (sm/md/lg/entry/entry-lg), radius scale, sidebar width
- Shadows: light preset (drop shadows), dark preset (glow borders)
- Spacing scale: 4px grid (space-0.5 through space-16)

Import at top of `index.css` before all component CSS.

#### 1.2 Rewrite `primitives/tokens.rs`

```rust
/// Semantic variant — expresses intent, not visual style.
/// Token system maps each variant to colors.
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}
```

Remove: `Effect`, `Shape`. These concepts don't exist in the design system.

Each enum needs a method to return its CSS class name string.

#### 1.3 Evaluate `primitives/color.rs`

`RgbColor` is used elsewhere in the app (theme system Rust backend). **Keep the file but remove it from the component prop API.** Components must never accept a color prop.

Check if `RgbColor` is used outside `ui/` — if so, it stays in `primitives/` but is no longer imported by components. If only used by components, move it or remove it.

---

### Phase 2: Button Component

#### 2.1 Rewrite `button.rs`

Props (matching spec exactly):

```rust
#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] variant: Variant,           // default: Secondary
    #[prop(optional)] size: Size,                 // default: Md
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional, default = false)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,  // button/submit/reset
    #[prop(optional)] leading_icon: Option<Children>,   // 16×16 icon slot
    #[prop(optional)] trailing_icon: Option<Children>,  // not on danger
    #[prop(optional, default = false)] full_width: bool,
    #[prop(optional, default = "")] class: &'static str, // layout overrides only
    #[prop(optional)] on_click: Option<Callback<web_sys::MouseEvent>>,
    #[prop(optional, default = "")] aria_label: &'static str,
) -> impl IntoView
```

Behavior:
- When `disabled` or `loading`: suppress onClick, add `disabled` HTML attribute
- When `loading`: set `aria-busy="true"`, replace leading_icon with spinner, keep label visible, opacity 0.7
- CSS class composition: `btn btn-{variant} btn-{size}` + optional `btn-full-width` + user `class`
- No inline style — all styling via CSS classes that reference semantic tokens

#### 2.2 Rewrite `button/styles.rs`

Simple variant/size → class name mapping:

```rust
impl Variant {
    pub fn button_class(&self) -> &'static str {
        match self {
            Variant::Primary => "btn-primary",
            Variant::Secondary => "btn-secondary",
            Variant::Ghost => "btn-ghost",
            Variant::Danger => "btn-danger",
            Variant::Warning => "btn-warning",
        }
    }
}

impl Size {
    pub fn button_class(&self) -> &'static str {
        match self {
            Size::Sm => "btn-sm",
            Size::Md => "btn-md",
            Size::Lg => "btn-lg",
        }
    }
}
```

#### 2.3 Rewrite `button.css`

Complete CSS using semantic tokens. Per-variant × per-state token mapping:

```css
.btn {
    /* Base */
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;                           /* icon gap */
    border-radius: var(--radius);       /* 6px always */
    font-family: var(--font-sans);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color var(--duration-fast) var(--ease-out),
                color var(--duration-fast) var(--ease-out),
                border-color var(--duration-fast) var(--ease-out);
    user-select: none;

    /* Focus — instant, never hidden */
    &:focus-visible {
        outline: 2px solid var(--color-focus-ring);
        outline-offset: 2px;
        transition: none;
    }

    /* Active — micro press */
    &:active:not(:disabled) {
        transform: scale(0.97);
        transition: transform var(--duration-micro) var(--ease-out);
    }

    /* Disabled */
    &:disabled {
        opacity: 0.4;
        cursor: not-allowed;
        pointer-events: none;
    }
}

/* Sizes */
.btn-sm { height: var(--height-sm); font-size: var(--text-xs); padding-inline: 10px; }
.btn-md { height: var(--height-md); font-size: var(--text-sm); padding-inline: 16px; }
.btn-lg { height: var(--height-lg); font-size: var(--text-base); padding-inline: 20px; }

/* Variants — each maps to semantic tokens */
.btn-primary {
    background: var(--color-primary);
    color: var(--color-primary-foreground);
    &:hover:not(:disabled) {
        background: var(--color-primary-hover);
    }
}

.btn-secondary {
    background: var(--color-surface-1);
    color: var(--color-text-primary);
    border-color: var(--color-border);
    &:hover:not(:disabled) {
        background: var(--color-surface-2);
    }
}

.btn-ghost {
    background: transparent;
    color: var(--color-text-secondary);
    &:hover:not(:disabled) {
        background: var(--color-surface-2);
        color: var(--color-text-primary);
    }
}

.btn-danger {
    background: var(--color-danger);
    color: var(--color-danger-foreground);
    &:hover:not(:disabled) {
        background: var(--color-danger-hover);
    }
}

.btn-warning {
    background: var(--color-warning);
    color: var(--color-warning-foreground);
    &:hover:not(:disabled) {
        background: var(--color-warning-hover);
    }
}

/* Full width */
.btn-full-width { width: 100%; }

/* Loading */
.btn-loading { opacity: 0.7; pointer-events: none; }
```

---

### Phase 3: IconButton Component

#### 3.1 Rewrite `icon_button.rs`

Props (matching spec exactly):

```rust
#[component]
pub fn IconButton(
    icon: Children,                                    // required icon slot
    #[prop(into)] aria_label: String,                  // REQUIRED — never optional
    #[prop(optional)] variant: Variant,                // default: Ghost (not Secondary!)
    #[prop(optional)] size: Size,                      // default: Md
    #[prop(optional, default = "")] tooltip: &'static str, // override tooltip text
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional, default = false)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional, default = "")] class: &'static str,
    #[prop(optional)] on_click: Option<Callback<web_sys::MouseEvent>>,
) -> impl IntoView
```

Note: default variant is `Ghost`, not `Secondary` (differs from Button).

Behavior:
- Square geometry enforced via CSS (width = height)
- `title` attribute set to tooltip if provided, else aria_label
- Icon centered via flexbox
- Dev-mode warning if aria_label is empty
- When loading: icon replaced by spinner, aria-busy="true"

#### 3.2 Rewrite `icon-button.css`

```css
.icon-btn {
    /* Same base as .btn */
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius);
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color var(--duration-fast) var(--ease-out),
                color var(--duration-fast) var(--ease-out);
    user-select: none;

    /* Square geometry */
    aspect-ratio: 1;

    /* Focus/Active/Disabled — same as .btn */
    ...
}

/* Sizes — square */
.icon-btn-sm { height: var(--height-sm); width: var(--height-sm); /* 14px icon */ }
.icon-btn-md { height: var(--height-md); width: var(--height-md); /* 16px icon */ }
.icon-btn-lg { height: var(--height-lg); width: var(--height-lg); /* 20px icon */ }

/* Variants — same token mapping as Button */
.icon-btn-primary { ... }
.icon-btn-secondary { ... }
.icon-btn-ghost { ... }  /* default */
.icon-btn-danger { ... }
.icon-btn-warning { ... }
```

---

### Phase 4: Cleanup

#### 4.1 Remove Ripple System
- Delete `ui/src/components/utilities/ripple/` directory
- Delete `ui/src/styles/ripple.css`
- Remove ripple import from `index.css`
- Remove ripple import from component files
- Clean up `utilities/mod.rs`

#### 4.2 Update Module Structure
- Update `components/foundation/mod.rs` — re-exports
- Update `components/mod.rs` — remove unused modules if empty
- Update `primitives/mod.rs` — keep color.rs if used elsewhere, update tokens.rs export

#### 4.3 Update Consumers
- Search the app crate for any usage of the old Button/IconButton API
- Update all call sites to use the new prop API
- Remove any `color=` props, `effect=` props, `shape=` props from templates

---

## File Change Summary

| File | Action |
|------|--------|
| `ui/src/styles/tokens.css` | **CREATE** — master design token file |
| `ui/src/styles/index.css` | **EDIT** — add tokens.css import, remove ripple.css |
| `ui/src/styles/button.css` | **REWRITE** — semantic tokens, 5 variants, all states |
| `ui/src/styles/icon-button.css` | **REWRITE** — semantic tokens, square geometry |
| `ui/src/styles/ripple.css` | **DELETE** |
| `ui/src/primitives/tokens.rs` | **REWRITE** — Variant(5 semantic), Size(3), remove Effect/Shape |
| `ui/src/primitives/color.rs` | **KEEP** (if used by theme backend) or **REMOVE** |
| `ui/src/components/foundation/button.rs` | **REWRITE** — new prop API, no color prop |
| `ui/src/components/foundation/button/styles.rs` | **REWRITE** — variant/size → CSS class |
| `ui/src/components/foundation/icon_button.rs` | **REWRITE** — icon slot, aria-label required, ghost default |
| `ui/src/components/foundation/icon_button/styles.rs` | **REWRITE** — variant/size → CSS class |
| `ui/src/components/utilities/ripple/` | **DELETE** entire directory |
| `ui/src/components/utilities/mod.rs` | **EDIT** — remove ripple module |
| Consumer call sites (app crate) | **UPDATE** — new API |

---

## Principles During Implementation

1. **No color props** — components read semantic CSS tokens only
2. **No inline styles** — all styling via CSS classes
3. **All 5 states required** — default, hover, focus, active, disabled (+ loading for interactive)
4. **Focus ring instant** — 0ms transition, 2px solid, never hidden
5. **Semantic token naming** — `--color-primary`, not `--background-color`
6. **Compound over prop-heavy** — use slots (leadingIcon, trailingIcon, icon) not render props
7. **Accessibility first** — native `<button>`, HTML disabled, aria-label, aria-busy
8. **Token system decides values** — components decide behavior, consumers decide intent
