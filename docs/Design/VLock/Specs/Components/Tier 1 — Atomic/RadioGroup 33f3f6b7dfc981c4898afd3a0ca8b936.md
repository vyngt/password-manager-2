# RadioGroup

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A group of mutually exclusive options where exactly one can be selected at a time. RadioGroup is the correct component when the option set is small (2–5 items), all options benefit from being visible simultaneously, and the choice is a primary decision — not a secondary configuration hidden in a dropdown.

If the option set exceeds ~5 items, use Select. If the user can select multiple options, use Checkbox.

---

## The three ownership boundaries

**The token system decides:** radio button colors, border, focus ring, and typography for option labels.

**The RadioGroup decides:** arrow-key navigation between options (not Tab), which radio is in the tab order at any time (roving tabindex), and how `orientation` maps to layout direction.

**The consumer decides:** `value`, `options`, `orientation`, `disabled`, and `onChange`.

---

## Anatomy

```
[radiogroup-root — role="radiogroup", fieldset or div]
  [radio-item × n]
    [hidden native <input type="radio">]
    [visual-circle — outer ring + inner dot]
    [label text — role.label typography]
```

Each radio item is a `<label>` wrapping a hidden native `<input type="radio">` and a custom visual circle. The visual circle is purely decorative — the native input provides all keyboard and screen reader behavior.

---

## Orientations

**`vertical`** (default) — Options stacked vertically, `gap: 8px`. Use for most form contexts.

**`horizontal`** — Options in a row, `gap: 16px`. Use for short option labels (2–3 words max) where horizontal layout saves vertical space without wrapping.

---

## Radio button anatomy

```
outer ring: 16×16px circle, border 1.5px
inner dot:  6×6px filled circle, centered
gap:        8px between radio button and label text
```

**Unchecked:** Outer ring only. `border: 1.5px solid --color-border-strong`. No inner dot.

**Checked:** Outer ring `border: 1.5px solid --color-primary`. Inner dot `background: --color-primary`, scales in with spring easing.

**Hover:** Outer ring border shifts to `--color-primary` even when unchecked — previews the selection color.

**Focus:** `outline: 2px solid --color-focus-ring`, offset 2px, on the visual circle. Instant (0ms).

**Disabled:** `opacity: 0.4` on the individual radio item. `pointer-events: none`.

---

## Motion

```
Inner dot appear:
  scale(0) → scale(1.2) → scale(1)
  duration: var(--duration-fast) 150ms
  easing:   var(--ease-spring)

Outer ring border-color:
  duration: var(--duration-fast) 150ms
  easing:   var(--ease-out)
```

---

## Keyboard navigation — roving tabindex

RadioGroup uses **roving tabindex** — only the selected option (or first option if none selected) is in the tab order (`tabindex="0"`). All others are `tabindex="-1"`.

| Key | Action |
| --- | --- |
| Tab | Enters the group (focuses selected / first option) |
| ↑ / ← | Moves to previous option, selects it |
| ↓ / → | Moves to next option, selects it |
| Tab (from inside) | Exits the group entirely |

Arrow keys both **move focus and select** simultaneously — unlike a listbox where navigation and selection are separate. This is the correct ARIA radio group pattern.

Navigation wraps: ↑ from first goes to last, ↓ from last goes to first.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `options` | `{ value: string, label: string, disabled?: boolean }[]` | — | **Required.** Option list. Order determines render and keyboard navigation order. |
| `value` | `string` | — | Controlled selected value. |
| `defaultValue` | `string` | — | Uncontrolled default. |
| `orientation` | `"vertical" / "horizontal"` | `"vertical"` | Layout direction of the option list. |
| `disabled` | `boolean` | `false` | Disables the entire group. Individual options can also be disabled via the `options` array. |
| `onChange` | `(value: string) => void` | — | Fires when selection changes. |
| `aria-label` | `string` | — | Group label for screen readers. Use when the group has no visible `<legend>` or heading. |
| `aria-labelledby` | `string` | — | Points to an external heading or label element's ID. |
| `class` | `string` | — | Layout overrides only. |

---

## Token mapping

| State | Outer ring border | Inner dot |
| --- | --- | --- |
| unchecked | `--color-border-strong` | — |
| unchecked + hover | `--color-primary` | — |
| checked | `--color-primary` | `--color-primary` |
| checked + hover | `--color-primary-hover` | `--color-primary-hover` |
| disabled | any (opacity 0.4) | any (opacity 0.4) |

---

## Accessibility requirements

- **`role="radiogroup"`** on the root. Requires `aria-label` or `aria-labelledby`.
- **Native `<input type="radio">`** for each option — visually hidden, never `display:none`.
- **`name` attribute** same value on all inputs in the group — the browser enforces mutual exclusivity via this.
- **Roving tabindex** — exactly one `tabindex="0"` in the group at any time.
- **Wrap in `<fieldset>` + `<legend>`** in forms. The legend provides the group's accessible name and is announced before each radio option.

---

## Anti-patterns

**Using RadioGroup for more than 5 options.** Use Select — all options visible simultaneously stops being a benefit when the list is long.

**Allowing no selection.** RadioGroup should always have a default value. An "empty" radio group confuses users — they cannot tell if they missed something or nothing is required.

**Using Tab to navigate between options.** Roving tabindex is the correct pattern. Tab exits the group.