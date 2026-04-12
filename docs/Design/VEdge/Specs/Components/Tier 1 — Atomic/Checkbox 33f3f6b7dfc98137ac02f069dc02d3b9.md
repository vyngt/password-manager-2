# Checkbox

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A form selection control with three states: unchecked, checked, and indeterminate. Checkbox is for multi-select lists, form agreements, and parent-of-group selection. The indeterminate state exists specifically for a parent checkbox that controls a group where some — but not all — children are checked.

Checkbox is not Toggle. Checkbox represents a value in a submitted form or a selection in a list. Toggle controls a live setting.

---

## The three ownership boundaries

**The token system decides:** border color, fill color, radius, focus ring, and transition timing.

**The Checkbox decides:** how indeterminate state renders (dash instead of check), and how the custom visual element mirrors the hidden native input's state.

**The consumer decides:** `checked`, `indeterminate`, `disabled`, `size`, and `onChange`.

---

## Anatomy

```
[checkbox-root — label or div, click area]
  [hidden native <input type="checkbox">]
  [visual-box — custom styled square]
    [check-icon or dash-icon — SVG, currentColor]
```

The native input is visually hidden (not `display:none` — it must remain in the DOM for keyboard and screen reader access). The visual box reflects the input's state via CSS sibling selectors.

---

## Sizes

| Size | Box | Icon stroke | Use |
| --- | --- | --- | --- |
| `sm` | 14×14px | 1.5px | Dense tables, compact lists |
| `md` | 16×16px | 2px | Default. Forms, settings, standard lists. |

Checkbox does not have a `lg` size. A 20px+ checkbox competes visually with body text and rarely serves a real use case.

---

## States

**Unchecked:** Border 1px `--color-border-strong`. Background transparent.

**Checked:** Background `--color-primary`. Border transparent. White checkmark SVG inside.

**Indeterminate:** Background `--color-primary`. Border transparent. White dash (−) SVG inside. Programmatically set via `indeterminate` prop — CSS `:indeterminate` alone is not reliable cross-browser.

**Hover:** Border strengthens to `--color-primary` (unchecked) or background shifts to `--color-primary-hover` (checked/indeterminate). Signals interactivity.

**Focus:** `outline: 2px solid --color-focus-ring`, `outline-offset: 2px` on the visual box. Appears at 0ms (instant).

**Disabled:** `opacity: 0.4` on root. `pointer-events: none`. `cursor: not-allowed`.

---

## Motion

```
Background fill:
  duration:  var(--duration-fast)  150ms
  easing:    var(--ease-out)

Check/dash icon:
  enter: opacity 0 → 1 + scale(0.8) → scale(1)
  duration: var(--duration-fast)  150ms
  easing:   var(--ease-spring)    cubic-bezier(0.34, 1.56, 0.64, 1)
```

The spring easing on the check icon gives a satisfying "snap" on selection. Same pattern as copy confirmation checkmark in the motion framework.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `checked` | `boolean` | `false` | Controlled checked state. |
| `indeterminate` | `boolean` | `false` | Partial-selection state. Overrides `checked` visually. Set programmatically when a parent checkbox controls a partially-selected group. |
| `defaultChecked` | `boolean` | `false` | Uncontrolled default. |
| `size` | `"sm" / "md"` | `"md"` | Physical scale. |
| `disabled` | `boolean` | `false` | Via HTML `disabled`. Opacity 0.4, removed from tab order. |
| `onChange` | `(checked: boolean) => void` | — | Fires on change. Does not fire when `indeterminate` changes — that is controlled entirely by the consumer. |
| `aria-label` | `string` | — | Required when no visible adjacent label. |
| `class` | `string` | — | Layout overrides only. |

---

## Token mapping

| State | Background | Border | Icon |
| --- | --- | --- | --- |
| unchecked | transparent | `--color-border-strong` | — |
| unchecked + hover | transparent | `--color-primary` | — |
| checked | `--color-primary` | transparent | white `#ffffff` |
| checked + hover | `--color-primary-hover` | transparent | white `#ffffff` |
| indeterminate | `--color-primary` | transparent | white `#ffffff` (dash) |
| disabled | any (opacity 0.4) | any (opacity 0.4) | — |

---

## CSS implementation reference

```css
.checkbox { position: relative; display: inline-flex; align-items: center; cursor: pointer; }

.checkbox__input {
  position: absolute; opacity: 0; width: 0; height: 0;
}

.checkbox__box {
  display: flex; align-items: center; justify-content: center;
  border-radius: var(--radius-sm); /* 4px */
  border: 1px solid var(--color-border-strong);
  background: transparent;
  transition: background-color var(--duration-fast) var(--ease-out),
              border-color     var(--duration-fast) var(--ease-out);
  flex-shrink: 0;
}
.checkbox--sm .checkbox__box { width: 14px; height: 14px; }
.checkbox--md .checkbox__box { width: 16px; height: 16px; }

/* Checked */
.checkbox__input:checked ~ .checkbox__box,
.checkbox--indeterminate .checkbox__box {
  background: var(--color-primary);
  border-color: transparent;
}

/* Hover */
.checkbox:hover:not(.checkbox--disabled) .checkbox__input:not(:checked) ~ .checkbox__box {
  border-color: var(--color-primary);
}
.checkbox:hover:not(.checkbox--disabled) .checkbox__input:checked ~ .checkbox__box,
.checkbox:hover:not(.checkbox--disabled).checkbox--indeterminate .checkbox__box {
  background: var(--color-primary-hover);
}

/* Focus */
.checkbox__input:focus-visible ~ .checkbox__box {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}

/* Icon */
.checkbox__icon {
  color: #ffffff;
  animation: checkbox-pop var(--duration-fast) var(--ease-spring) forwards;
}
@keyframes checkbox-pop {
  from { opacity: 0; transform: scale(0.8); }
  to   { opacity: 1; transform: scale(1); }
}

/* Disabled */
.checkbox--disabled { opacity: 0.4; cursor: not-allowed; pointer-events: none; }

@media (prefers-reduced-motion: reduce) {
  .checkbox__box { transition-duration: 0.01ms !important; }
  .checkbox__icon { animation-duration: 0.01ms !important; }
}
```

---

## Indeterminate — implementation note

The CSS `:indeterminate` pseudo-class exists but is inconsistent across browsers when set programmatically. Always set `input.indeterminate = true` via JavaScript AND apply a CSS class (`.checkbox--indeterminate`) to the root for reliable styling:

```jsx
// On mount and on prop change:
if (inputRef.current) {
  inputRef.current.indeterminate = props.indeterminate;
}
```

---

## Accessibility requirements

- **Use native `<input type="checkbox">`** — visually hidden, never `display:none`.
- **`aria-checked="mixed"`** when indeterminate. Screen readers announce "mixed" for partial selection.
- **Label association.** Wrap in `<label>` or use `aria-labelledby`.
- **Group context.** When checkboxes belong to a group, wrap in `<fieldset>` with `<legend>`. The legend provides the group's accessible name.

---

## Anti-patterns

**Using Checkbox for a live on/off setting.** That is Toggle.

**Faking indeterminate with a custom icon.** Set `input.indeterminate = true` programmatically. CSS-only approaches break screen reader announcements.

**Checkbox without a label.** Every checkbox must have an accessible name — label element, `aria-label`, or `aria-labelledby`.