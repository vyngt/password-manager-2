# Color Token Framework

### A principled system for themeable, accessible, predictable UI color

---

## Why a token system at all

Hardcoding colors creates a maintenance problem that compounds over time. When a designer says "make the primary button slightly darker," the change should happen in one place and propagate everywhere. When a user customizes their theme, no component should require special casing. When a new brand color is chosen, the entire interface should adapt without a single line of component code changing.

A token system solves this by introducing a **named layer of indirection** between raw color values and the UI that uses them. Components never reference `#2563EB` — they reference `--color-primary`. What `--color-primary` resolves to is the theme's concern, not the component's.

---

## The two-layer model

### Layer 1 — Primitive tokens

Raw values. No meaning, only value. **Never used directly in components.**

```
primitive.zinc.950   = #09090B
primitive.zinc.900   = #18181B
primitive.zinc.50    = #FAFAFA
primitive.blue.600   = #2563EB
primitive.red.600    = #DC2626
```

### Layer 2 — Semantic tokens

Named by role, not by value. These are what components actually consume.

```
color.background          → resolves to a primitive
color.surface.1           → resolves to a primitive
color.text.primary        → resolves to a primitive
color.primary             → resolves to a primitive
color.danger              → resolves to a primitive
```

Swapping a theme means replacing what the semantic tokens resolve to. The component code stays unchanged.

---

## What users can control

The color system has two dimensions that are independent of each other: **token structure** and **user configurability**. These must not be conflated.

**Token structure** is identical for every color group. Each group — primary, danger, warning, success — exposes the same five tokens:

```
{group}            base color
{group}-hover      hover state
{group}-muted      soft tint for badges and highlights
{group}-foreground text on {group} background
{group}-text       {group} text on neutral surfaces
```

This uniformity means components can be built against a single contract. A `Badge` component doesn't need to know whether the color group is user-configurable or locked — it just reads the tokens.

**User configurability** is a backend concern, enforced in the settings UI. Each color group in `ThemeConfig` carries a flag:

```rust
struct ColorGroup {
    base:       Color,
    hover:      Color,
    muted:      Color,
    foreground: Color,
    text:       Color,
    user_configurable: bool,   // false = Settings UI hides this field
}

// primary      → user_configurable: true
// background   → user_configurable: true
// foreground   → user_configurable: true
// danger       → user_configurable: false
// warning      → user_configurable: false
// success      → user_configurable: false
```

The CSS token names, Tailwind utilities, and component code are identical regardless of the flag. If the team decides to allow warning color customization, the change is one line in the config — no component code changes.

---

## Deriving the full palette

### Surface layers

Derived by mixing `--root-background` toward `--root-foreground` in increasing increments:

```
surface.base       = root-background                    (page bg)
surface.1          = mix(background → foreground,  4%)  (sidebar, card, panel)
surface.2          = mix(background → foreground,  8%)  (hover state)
surface.3          = mix(background → foreground, 14%)  (selected / active)
surface.4          = mix(background → foreground, 22%)  (popover, modal)

border.default     = mix(background → foreground, 12%)
border.strong      = mix(background → foreground, 24%)
```

This works in both directions — light or dark backgrounds self-correct automatically.

### Primary scale

From a single `--root-primary`, the system generates ten tonal steps:

```
primary.50    very light tint  → badge background, subtle highlight
primary.600   base color       → interactive default ← root-primary
primary.700   slightly darker  → hover on interactive elements
primary.800   dark             → text color on light surfaces
```

Tints (50–500): mix toward `--root-background`. Shades (700–900): mix toward near-black. Root sits at 600 — contrast against white first exceeds 4.5:1 for most hues here.

### Text color tiers

```
text.primary    = root-foreground
text.secondary  = mix(foreground, background, 30%)
text.tertiary   = mix(foreground, background, 55%)
```

---

## The one automatic decision: text-on-primary

When text sits on a `primary.600` background, the system computes whether white or black produces higher contrast — this cannot be left to user discretion since primary is user-controlled.

```
primary_foreground(primary_color):
  lum = relative_luminance(primary_color)
  white_contrast = contrast_ratio(1.0, lum)
  return white_contrast ≥ 4.5 ? #ffffff : #111827
```

Stored as `--color-primary-foreground`. Components always read from the token, never decide.

---

## What opacity is and is not for

**Do not use opacity for surfaces.** Semi-transparent colors produce unpredictable results when backgrounds change.

Opacity is permitted in exactly two contexts:

- **Modal scrim** — `rgba(0, 0, 0, 0.5)` intentionally darkens behind the dialog
- **Vibrancy / frosted glass** — macOS/Windows 11 sidebar effect, must fall back to `surface.1`

---

## Validation and user safety

Validation runs in the Rust backend, not CSS. Three checks on every theme change:

```
Check 1: body text readability
  contrast(text.primary, surface.base)    ≥ 7.0   (AAA target)
  contrast(text.secondary, surface.base)  ≥ 4.5   (AA minimum)

Check 2: interactive text readability
  contrast(primary-foreground, primary.600) ≥ 4.5

Check 3: focus visibility
  contrast(focus-ring, adjacent-surface)  ≥ 3.0
```

Ratio < 3.0 → blocks save. 3.0–4.5 → saves with warning. > 4.5 → clean.

---

## Token naming convention

| Conceptual name | CSS custom property |
| --- | --- |
| `color.background` | `--color-background` |
| `color.surface.1` | `--color-surface-1` |
| `color.surface.2` | `--color-surface-2` |
| `color.surface.3` | `--color-surface-3` |
| `color.surface.4` | `--color-surface-4` |
| `color.text.primary` | `--color-text-primary` |
| `color.text.secondary` | `--color-text-secondary` |
| `color.text.tertiary` | `--color-text-tertiary` |
| `color.primary` | `--color-primary` |
| `color.primary.hover` | `--color-primary-hover` |
| `color.primary.muted` | `--color-primary-muted` |
| `color.primary.foreground` | `--color-primary-foreground` |
| `color.border` | `--color-border` |
| `color.border.strong` | `--color-border-strong` |
| `color.danger` | `--color-danger` |
| `color.danger.muted` | `--color-danger-muted` |
| `color.success` | `--color-success` |
| `color.warning` | `--color-warning` |
| `color.focus.ring` | `--color-focus-ring` |

Names describe **intent**, not value or position.

---

## Theme model

Themes are database records, not CSS modes. "Light" and "dark" are simply two default rows in the themes table — not special CSS modes. CSS has no knowledge of them.

### How it works

```
App startup:
  1. CSS fallback values render (~50ms, invisible to user)
  2. Rust reads active_theme_id from app database
  3. Rust loads that ThemeConfig row, computes CSS variables
  4. Rust injects <style> block on :root → done

First launch (no saved theme):
  Rust creates a default ThemeConfig row, sets as active → proceeds above

User switches theme:
  Rust loads selected row, injects, updates active_theme_id → done
```

### The Rust injection model

```rust
fn inject_theme(config: &ThemeConfig, window: &WebviewWindow) {
    let css = config.to_css_vars();
    window.eval(&format!(
        "document.head.insertAdjacentHTML('beforeend', '<style>{}</style>')", css
    )).ok();
}
```

No `[data-theme]`, no `prefers-color-scheme`, no CSS-level switching.

### What CSS holds

`tokens.css` defines neutral fallback values — not a "light preset." They exist only to prevent a flash of unstyled content before Rust injects.

### Reference values

**Default (light background):**

```
--color-background: #ffffff
--color-primary:    #2563EB   (blue-600)
--color-text-primary:   #111827   (15.4:1 — AAA)
--color-danger:     #DC2626
--shadow-md:        0 4px 12px rgba(0,0,0,0.12)
```

**Default (dark background):**

```
--color-background: #09090B   (zinc-950)
--color-primary:    #60A5FA   (blue-400)
--color-text-primary:   #FAFAFA
--shadow-md:        0 0 0 1px rgba(255,255,255,0.10)
```

These values live in the database. The CSS file does not know about them.

### Tailwind color utilities

```html
<div class="bg-background">
<aside class="bg-surface-1 border border-border">
<div class="hover:bg-surface-2">
<p class="text-text-primary">
<button class="bg-primary text-primary-foreground hover:bg-primary-hover">
<button class="bg-danger text-danger-foreground">
<span class="bg-danger-muted text-danger-text">
```

```rust
fn inject_theme(config: &ThemeConfig, window: &WebviewWindow) {
    let css = config.to_css_vars();
    window.eval(&format!(
        "document.head.insertAdjacentHTML('beforeend', '<style>{}</style>')", css
    )).ok();
}
```

No `[data-theme]` attribute, no `prefers-color-scheme` media query, no CSS-level switching logic. Theme changes after startup: Rust recomputes, reinjects, saves.

### Token values per preset

**Light preset (fallback in `tokens.css`):**

```
--color-background: #ffffff
--color-primary:    #2563EB   (blue-600)
--color-text-primary:   #111827   (15.4:1 — AAA)
--color-text-secondary: #4B5563   (7.1:1  — AA)
--color-danger:     #DC2626   (red-600)
--shadow-md:        0 4px 12px rgba(0, 0, 0, 0.12)
```

**Dark preset (injected by Rust when config.theme == Dark):**

```
--color-background: #09090B   (zinc-950)
--color-primary:    #60A5FA   (blue-400 — lighter on dark bg)
--color-primary-foreground: #0F172A
--color-text-primary:   #FAFAFA
--color-text-secondary: #A1A1AA
--shadow-md:        0 0 0 1px rgba(255, 255, 255, 0.10)
```

### Tailwind color utilities

```html
<div class="bg-background">
<aside class="bg-surface-1 border border-border">
<div class="hover:bg-surface-2">
<p class="text-text-primary">
<span class="text-text-secondary">
<button class="bg-primary text-primary-foreground hover:bg-primary-hover">
<button class="bg-danger text-danger-foreground">
<span class="bg-danger-muted text-danger-text">
```

---

## Summary of rules

1. Components reference semantic tokens only — never primitives, never raw hex.
2. Semantic tokens are derived; not chosen arbitrarily.
3. All color groups follow the same five-token structure: base / hover / muted / foreground / text.
4. Configurability is a backend concern. Token structure is identical for all groups.
5. `primary-foreground` is always computed from contrast math, never manually set.
6. Opacity is not used for surfaces. Only for modal scrims and platform vibrancy.
7. Validation runs before any theme is applied. Insufficient contrast blocks the change.
8. Theme switching is a database + Rust concern. CSS is theme-blind — it holds neutral fallback values only.
9. Token names describe intent. Never value, position, or appearance.