# Button

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

The primary interactive primitive. Every clickable action in the interface that does not navigate to a new route is a Button. It is the most frequently rendered component in the system and the one that exercises the most complete cross-section of the token stack — color, typography, spacing, motion, elevation, and accessibility all converge here.

Get the Button spec right and the token system is validated. Get it wrong and every subsequent component inherits the inconsistency.

---

## The three ownership boundaries

Before any implementation detail: every visual property of the Button belongs to exactly one owner.

**The token system decides:** surface colors, text color tiers, border radius, duration values, easing curves, focus ring appearance. The Button never hardcodes these values. It reads CSS custom properties.

**The Button decides:** which surface level each variant occupies, how hover differs from default, what transitions are applied and in what order, how loading state restructures internal layout. These are internal rules — not exposed as props.

**The consumer decides:** semantic intent (`variant`), physical scale (`size`), disabled state, loading state, icon content, label text. The consumer always picks *intent*, never *value*. `variant="danger"` is correct. `color="#DC2626"` is not permitted.

---

## Variants

Five variants. Each communicates a different level of semantic weight. They are not interchangeable.

| Variant | Semantic intent | When to use |
| --- | --- | --- |
| `primary` | The single most important action on the current surface | Unlock vault, Save entry, Confirm. At most one primary visible at a time. |
| `secondary` | Supporting action | Cancel, Edit, Go back. Appears alongside primary. |
| `ghost` | Lowest emphasis, optional action | See details, Show more, tertiary navigation items. No background until hover. |
| `danger` | Destructive or irreversible | Delete entry, Revoke access, Wipe vault. Always paired with a cancel option 24px away minimum. |
| `warning` | Caution — consequential but reversible | Overwrite entry, Reset to default. Less visually heavy than danger by design. |

> **The `primary` constraint is strict.** Two primary buttons visible at the same time means neither is "most important." If two actions feel equal in weight, both should be `secondary`.
> 

> **The `danger` pairing rule is non-negotiable.** The gap between danger and its paired cancel must be at least 24px — not a visual preference, a cognitive protection.
> 

---

## Sizes

Three sizes. Heights are not chosen — they fall out of the typography system.

```
height = line-height(font-size) + (2 × padding-y)

sm:  28px = 16px (text-xs)   + (2 × 6px)
md:  36px = 20px (text-sm)   + (2 × 8px)   ← default
lg:  44px = 24px (text-base) + (2 × 10px)
```

| Size | Height | Font | Padding X | Use |
| --- | --- | --- | --- | --- |
| `sm` | 28px | 12px | 10px | Compact tables, dense lists, inline actions |
| `md` | 36px | 14px | 16px | Default. Use when no size prop is specified. |
| `lg` | 44px | 16px | 20px | Primary CTA: Unlock vault, Submit, Continue |

---

## States

All six states are required. Skipping a state is a bug.

**Default** — Resting state. All variant token values at their base level.

**Hover** — Background shifts one surface step. Transition: `background-color 150ms ease-out`. Never change size, position, padding, or layout on hover.

**Focus** — Keyboard focus. Ring appears instantly at 0ms — a delayed ring is broken. Spec: `outline: 2px solid --color-focus-ring`, `outline-offset: 2px`. Uses `--color-focus-ring` (not `--color-primary` directly) so custom themes can diverge the two values independently. Never hidden or suppressed for any variant.

**Active** — Element is being pressed. `transform: scale(0.97)`, 80ms ease-out. Reverts on release with the same 80ms.

**Disabled** — `opacity: 0.4`, `cursor: not-allowed`, `pointer-events: none`. Applied via the HTML `disabled` attribute, not CSS class alone. Appears instantly (0ms).

**Loading** — Implies disabled. `opacity: 0.7`. Leading icon replaced by spinner. Label text stays visible — never spinner-only. Spinner: 13px (sm/md), 16px (lg), 700ms `linear`.

---

## Icon support

Buttons accept `leadingIcon` (before label) and `trailingIcon` (after label). Each is a `Slot` — a framework's unit of renderable content. Expected: 16×16 SVG with `stroke="currentColor"` and `stroke-width="2"`.

Internal gap: **8px** between icon and text (`--space-2`). 7px is not on the 4px grid — the spacing framework requires all gaps to be multiples of 4px.

```jsx
[leadingIcon 16×16] [8px] [label text] [8px] [trailingIcon 16×16]
```

When `loading` is true, `leadingIcon` is replaced by the spinner. Trailing icon is **not permitted on `danger` buttons** — it introduces ambiguity about the destructive weight.

---

## Full width

`fullWidth` sets `width: 100%`. Reserved for the primary CTA in an unlock screen, login form, or single-action modal. Do not use for inline actions.

---

## Props API

Types use abstract notation. `Slot` = framework's renderable content. `EventHandler<T>` = callback receiving that event. See Framework adaptation section below.

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `variant` | `"primary" / "secondary" / "ghost" / "danger" / "warning"` | `"secondary"` | Semantic intent. Consumer chooses intent, never a color value. |
| `size` | `"sm" / "md" / "lg"` | `"md"` | Physical scale. Height is derived from the typography system — not chosen independently. |
| `disabled` | `boolean` | `false` | Via HTML `disabled` attribute, not CSS class alone. Removes from tab order. |
| `loading` | `boolean` | `false` | Implies disabled. Replaces leadingIcon with spinner. Label stays visible. |
| `type` | `"button" / "submit" / "reset"` | `"button"` | HTML button type. Always set explicitly — the HTML default is `"submit"`, which fires on Enter in any parent form. |
| `leadingIcon` | `Slot` | — | 16×16 icon before label. Replaced by spinner when loading. |
| `trailingIcon` | `Slot` | — | 16×16 icon after label. Not permitted on `danger` variant. |
| `fullWidth` | `boolean` | `false` | `width: 100%`. Reserved for primary CTAs only. |
| `class` | `string` | — | Layout overrides only: margin, width, flex. Visual overrides are not permitted. |
| `onClick` | `EventHandler<MouseEvent>` | — | Suppressed automatically when `disabled` or `loading`. See Framework adaptation for per-framework naming. |
| `aria-label` | `string` | — | Required when label text is ambiguous in context (e.g. multiple "Delete" buttons on one screen). |

---

## Token mapping

### By variant

| Variant | State | Background | Text | Border |
| --- | --- | --- | --- | --- |
| `primary` | default | `--color-primary` | `--color-primary-foreground` | transparent |
| `primary` | hover | `--color-primary-hover` | `--color-primary-foreground` | transparent |
| `secondary` | default | `--color-surface-1` | `--color-text-primary` | `--color-border` |
| `secondary` | hover | `--color-surface-2` | `--color-text-primary` | `--color-border` |
| `ghost` | default | transparent | `--color-text-secondary` | transparent |
| `ghost` | hover | `--color-surface-2` | `--color-text-primary` | transparent |
| `danger` | default | `--color-danger` | `--color-danger-foreground` | transparent |
| `danger` | hover | `--color-danger-hover` | `--color-danger-foreground` | transparent |
| `warning` | default | `--color-warning` | `--color-warning-foreground` | transparent |
| `warning` | hover | `--color-warning-hover` | `--color-warning-foreground` | transparent |

### By state (all variants)

| State | Properties | Duration | Easing |
| --- | --- | --- | --- |
| focus | `outline: 2px solid --color-focus-ring`, offset 2px | **0ms** | — |
| active | `transform: scale(0.97)` | 80ms | ease-out |
| disabled | `opacity: 0.4` on root | 0ms | — |
| loading | `opacity: 0.7` on root | 0ms | — |
| hover bg | `background-color` | 150ms | ease-out |

### Motion tokens consumed

```
--duration-micro   80ms    active press/release
--duration-fast    150ms   hover transitions
--ease-out         cubic-bezier(0.16, 1, 0.3, 1)
```

### Typography tokens consumed

```
--font-sans
--font-weight-semibold   (600)
--text-xs / --text-sm / --text-base   (by size)
```

### Spacing tokens consumed

```jsx
Border radius:   6px
Padding X:       10px / 16px / 20px  (sm / md / lg)
Icon gap:        8px   (--space-2)
```

---

## CSS implementation reference

This shows one implementation using plain CSS / BEM. Any styling approach that reads the token system's CSS custom properties is correct — plain CSS, CSS Modules, Tailwind (via arbitrary values or theme extension), etc.

**Tailwind note:** map tokens into the Tailwind config so utility classes resolve to CSS variables. Avoid `bg-[#2563EB]` — hardcoding hex bypasses the token system the same way it does in plain CSS.

```css
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px; /* --space-2: must be on 4px grid */
  border-radius: 6px;
  border: 0.5px solid transparent;
  font-family: var(--font-sans);
  font-weight: var(--font-weight-semibold);
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
  outline: none;
  transition:
    background-color var(--duration-fast) var(--ease-out),
    filter          var(--duration-fast) var(--ease-out),
    border-color    var(--duration-fast) var(--ease-out),
    color           var(--duration-fast) var(--ease-out),
    transform       var(--duration-micro) var(--ease-out),
    box-shadow      var(--duration-fast) var(--ease-out);
}

.btn--sm { height: 28px; padding: 0 10px; font-size: var(--text-xs); }
.btn--md { height: 36px; padding: 0 16px; font-size: var(--text-sm); }
.btn--lg { height: 44px; padding: 0 20px; font-size: var(--text-base); }

.btn--primary   { background: var(--color-primary);  color: var(--color-primary-foreground); }
.btn--secondary { background: var(--color-surface-1); color: var(--color-text-primary); border-color: var(--color-border); }
.btn--ghost     { background: transparent; color: var(--color-text-secondary); }
.btn--danger    { background: var(--color-danger);    color: var(--color-danger-foreground); }
.btn--warning   { background: var(--color-warning);   color: var(--color-warning-foreground); }

.btn--primary:hover:not(:disabled)   { background: var(--color-primary-hover); }
.btn--secondary:hover:not(:disabled) { background: var(--color-surface-2); }
.btn--ghost:hover:not(:disabled)     { background: var(--color-surface-2); color: var(--color-text-primary); }
.btn--danger:hover:not(:disabled)    { background: var(--color-danger-hover); }
.btn--warning:hover:not(:disabled)   { background: var(--color-warning-hover); }

.btn:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}

.btn:active:not(:disabled):not(.btn--loading) {
  transform: scale(0.97);
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

.btn--loading {
  opacity: 0.7;
  pointer-events: none;
}

@keyframes btn-spin { to { transform: rotate(360deg); } }
.btn__spinner {
  width: 13px; height: 13px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: btn-spin 700ms linear infinite;
  flex-shrink: 0;
}
.btn--lg .btn__spinner { width: 16px; height: 16px; }

.btn--full { width: 100%; }

@media (prefers-reduced-motion: reduce) {
  .btn { transition-duration: 0.01ms !important; }
  .btn__spinner { animation-duration: 0.01ms !important; }
}
```

---

## Composability

Button manages its own internal layout. It does not manage its position within a parent layout — that is always the consumer's responsibility.

```
// Correct — consumer owns positioning
container [display:flex, justify-content:flex-end, gap:24px]
  Button [variant=secondary] "Cancel"
  Button [variant=danger]    "Delete entry"

// Wrong — Button managing its own margin
Button [variant=danger, style="margin-left:auto"] "Delete entry"
```

Button's default slot accepts a string or inline element as the label. The slot must not contain other interactive elements — nested buttons and nested links produce invalid HTML.

**Inside a form:** always set `type="button"` on non-submit buttons. The HTML default is `type="submit"`, which means Enter in any text input fires the button.

---

## Framework adaptation

| Concept | React / Preact | Vue 3 | Svelte 5 | SolidJS | Leptos |
| --- | --- | --- | --- | --- | --- |
| Click handler | `onClick` | `@click` | `onclick` | `onClick` | `on:click` |
| Class override | `className` | `class` | `class` | `class` | `class` |
| Slot type | `ReactNode` | `VNode / Slot` | `Snippet` | `JSX.Element` | `Children` |
| Default slot | `children` prop | `<slot>` | `{@render children()}` | `props.children` | `children()` |
| Dynamic boolean | `loading={val}` | `:loading="val"` | `loading={val}` | `loading={val()}` | `loading=val` |

---

## Accessibility requirements

- **Keyboard operability.** Use the native `<button>` element. Never implement a button with a `<div>` or `<span>` and a click handler.
- **Focus visibility.** 2px solid ring, visible in all variants and themes, 3:1 contrast minimum against adjacent surface (WCAG SC 1.4.11).
- **Disabled state.** Use the HTML `disabled` attribute — not only CSS. The attribute removes the element from tab order and announces state to screen readers.
- **Loading state.** Add `aria-busy="true"` to root. Keep label text visible so users know what action is in progress.
- **Ambiguous labels.** Multiple "Delete" buttons on one screen require unique `aria-label` per instance: `aria-label="Delete GitHub entry"`.
- **Color is not the only signal.** Danger must be communicated by label text, not only by red color.

---

## Anti-patterns

**Passing a color value as a prop.** `Button [color="red"]` bypasses the token system. Use `variant="danger"`.

**Overriding visual properties via `class`.** The `class` prop is for layout only. If visual appearance needs to change, extend the `variant` API.

**Two primary buttons on one surface.** Neither is the most important action. One must become `secondary`.

**Danger button without a paired cancel.** A danger button without an escape path is a UX trap. The 24px gap is a spatial guarantee, not a suggestion.

**Ghost for destructive actions.** `ghost` signals "optional." Destructive actions need visual weight.

**Animating width, height, or padding.** The button must not resize when content changes (loading, icon swap). Internal layout accommodates the change.

**Spinner-only loading.** The label must remain. Users need to know what action is in progress.

---

## Implementation checklist

- [ ]  All five variants render with correct token values in light and dark mode
- [ ]  All three sizes derive from the typography system (height = line-height + 2× padding-y)
- [ ]  Hover transitions complete in 150ms ease-out on all variants
- [ ]  Focus ring appears at 0ms on all variants
- [ ]  Active state scale(0.97) triggers at 80ms ease-out on mousedown and keydown
- [ ]  Disabled state sets HTML `disabled` attribute, not only CSS
- [ ]  Loading state: spinner visible, label visible, pointer-events suppressed, opacity 0.7
- [ ]  Spinner animation is 700ms linear
- [ ]  Leading icon replaced by spinner when loading and leadingIcon is provided
- [ ]  `type="button"` is the default
- [ ]  Danger button: trailing icon not accepted
- [ ]  `aria-busy="true"` applied to root when loading
- [ ]  `prefers-reduced-motion` suppresses all transitions and spinner
- [ ]  WCAG AA contrast (4.5:1) for text on background in both themes
- [ ]  Focus ring contrast: 3:1 against adjacent surface in both themes
- [ ]  Keyboard: Space and Enter both activate
- [ ]  Disabled buttons removed from tab order
- [ ]  No layout shift on loading state change
- [ ]  No layout shift on hover