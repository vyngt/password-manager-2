# SegmentedControl

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A mutually exclusive button group for switching between discrete modes. SegmentedControl is the correct component for view mode switching (list / table / card), layout toggles, and any UI mode where exactly one option is always active and the options are visible simultaneously.

SegmentedControl is not a form input. It does not submit a value. It controls immediate UI state — the effect is instant, no Save required. It is not RadioGroup (which is for form values) and not Toggle (which is binary).

---

## The three ownership boundaries

**The token system decides:** track background, segment colors, border radius, typography, and transition timing.

**The SegmentedControl decides:** the animated active indicator that slides between segments, how icon-only segments handle sizing, and how keyboard navigation works (arrow keys, not Tab).

**The consumer decides:** `options`, `value`, `size`, and `onChange`. The consumer never controls colors or animation.

---

## Anatomy

```
[control-root — track, the pill-shaped container]
  [segment × n — button[role="radio"], aria-checked]
    [icon?]     ← 16×16 SVG
    [label?]    ← text-sm medium
  [active-indicator — absolutely positioned, slides between segments]
```

The track is a rounded container with a subtle background (`--color-surface-2`). The active indicator is a white (or surface-1) pill that slides under the active segment, creating the illusion of selection sliding across the control.

Each segment is a `<button>` with `role="radio"` and `aria-checked`. The group root has `role="radiogroup"`. This is the correct ARIA pattern — it communicates mutual exclusivity without requiring native radio inputs.

---

## Sizes

| Size | Track height | Segment padding | Font | Icon |
| --- | --- | --- | --- | --- |
| `sm` | 28px | `0 10px` | `text-xs` 12px / medium | 14×14px |
| `md` | 32px | `0 12px` | `text-sm` 14px / medium | 16×16px |

SegmentedControl has no `lg` size — a taller control competes with primary actions it should not.

Track inset (padding around the active indicator): **2px** on all sides. The active indicator is 4px shorter than the track height.

---

## Segment variants

**Text only** — label string. Minimum segment width: 48px.

**Icon only** — 16×16 SVG, no label. Segment width = 32px (sm) / 36px (md). Requires `aria-label` on each segment.

**Icon + text** — icon left, label right, `gap: 6px`. Mixed icon+text and text-only segments in the same control are not permitted — all segments must follow the same pattern.

---

## Active indicator animation

The active indicator is the visual centerpiece of SegmentedControl. It must slide, not jump.

```
Active indicator:
  property:  transform: translateX()  +  width (if segments vary in width)
  duration:  var(--duration-fast)  150ms
  easing:    var(--ease-in-out)    cubic-bezier(0.45, 0, 0.55, 1)
```

`ease-in-out` because the indicator moves from one resting position to another — repositioning, not entering or exiting.

Implementation: position the indicator absolutely within the track, calculate its `left` offset and `width` from the active segment's `offsetLeft` and `offsetWidth`. Update on value change. CSS `transition` on `transform` and `width` handles the animation.

Do **not** animate `left` — use `transform: translateX()` to stay GPU-composited.

---

## States

**Inactive segment (default):** `color: --color-text-secondary`. Background transparent (indicator not behind it).

**Active segment:** `color: --color-text-primary`. Active indicator (`--color-surface-1` or white) slides underneath.

**Hover (inactive):** `color: --color-text-primary`. No background change — the indicator does not move on hover.

**Focus:** `outline: 2px solid --color-focus-ring`, offset 2px, on the focused segment button. Instant (0ms). Focus ring appears on individual segments, not the track.

**Disabled (entire control):** `opacity: 0.4` on root. `pointer-events: none`.

**Disabled (individual segment):** `opacity: 0.4` on that segment. Cannot be selected. Navigation skips it.

---

## Keyboard navigation

Same roving tabindex pattern as RadioGroup:

| Key | Action |
| --- | --- |
| Tab | Enters the control (focuses active segment) |
| ← / → | Moves to previous / next segment, activates it |
| Tab (from inside) | Exits the control |

Navigation wraps. Arrow keys both move and select simultaneously.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `options` | `{ value: string, label?: string, icon?: Slot, ariaLabel?: string, disabled?: boolean }[]` | — | **Required.** Min 2 options, max 5. All must have `label` or `icon` (or both). Icon-only options require `ariaLabel`. |
| `value` | `string` | — | **Required (controlled).** Always one value active. SegmentedControl has no empty state. |
| `size` | `"sm" / "md"` | `"md"` | Physical scale. |
| `disabled` | `boolean` | `false` | Disables the entire control. Individual segments can be disabled via `options`. |
| `onChange` | `(value: string) => void` | — | Fires when active segment changes. Always receives a value — SegmentedControl never fires with `null`. |
| `aria-label` | `string` | — | Accessible name for the control group. E.g. `"View mode"`. |
| `class` | `string` | — | Layout overrides only: width, margin. |

---

## Token mapping

```
Track background:            --color-surface-2
Active indicator background: --color-surface-1  (or #ffffff in light mode)
Active indicator shadow:     --shadow-sm         (subtle lift over track)
Inactive label color:        --color-text-secondary
Active label color:          --color-text-primary
Hover label color:           --color-text-primary
Border radius (track):       var(--radius)        6px
Border radius (indicator):   var(--radius-sm)     4px  (slightly tighter than track)
```

---

## CSS implementation reference

```css
.seg-control {
  position: relative;
  display: inline-flex;
  background: var(--color-surface-2);
  border-radius: var(--radius);
  padding: 2px;
  gap: 0;
}

.seg-control--sm { height: 28px; }
.seg-control--md { height: 32px; }

/* Active indicator */
.seg-indicator {
  position: absolute;
  top: 2px; bottom: 2px;
  background: var(--color-surface-1);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-sm);
  transition: transform var(--duration-fast) var(--ease-in-out),
              width    var(--duration-fast) var(--ease-in-out);
  pointer-events: none;
  /* left and width set by JS from active segment offsetLeft/offsetWidth */
}

/* Segments */
.seg-item {
  position: relative; z-index: 1;
  display: inline-flex; align-items: center; justify-content: center;
  gap: 6px;
  border: none; background: transparent;
  font-family: var(--font-sans);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  white-space: nowrap;
  transition: color var(--duration-fast) var(--ease-out);
  user-select: none;
}
.seg-control--sm .seg-item { padding: 0 10px; font-size: var(--text-xs); }
.seg-control--md .seg-item { padding: 0 12px; font-size: var(--text-sm); }

.seg-item[aria-checked="true"] { color: var(--color-text-primary); }
.seg-item:hover:not([aria-checked="true"]):not(:disabled) { color: var(--color-text-primary); }

.seg-item:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: -1px; /* inside the segment, not outside the track */
}

.seg-item:disabled { opacity: 0.4; cursor: not-allowed; pointer-events: none; }

.seg-control--disabled { opacity: 0.4; pointer-events: none; }

@media (prefers-reduced-motion: reduce) {
  .seg-indicator { transition-duration: 0.01ms !important; }
  .seg-item { transition-duration: 0.01ms !important; }
}
```

---

## Accessibility requirements

- **`role="radiogroup"`** on the track root. Requires `aria-label`.
- **`role="radio"`** on each segment button. **`aria-checked`** reflects active state.
- **Roving tabindex** — only active segment has `tabindex="0"`. Others are `tabindex="-1"`.
- **Icon-only segments require `aria-label`** per segment. The icon alone is not an accessible name.
- **Active indicator is `aria-hidden`** — it is purely visual.

---

## Anti-patterns

**More than 5 segments.** Beyond 5, the control becomes too wide and each segment too narrow to read. Use a Select or Tab component instead.

**No default value.** SegmentedControl always has an active segment. There is no "none selected" state — unlike RadioGroup which can start empty in a form.

**Mixed segment types.** All segments text, all segments icon, or all segments icon+text. Never mix text-only and icon-only in the same control.

**Using for form submission.** SegmentedControl controls UI state only. If the selection needs to be submitted or persisted as a form field value, use RadioGroup.