# Spinner

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

An animated loading indicator for operations with indeterminate duration. Spinner communicates "something is happening" without implying how long it will take or how much progress has been made. For operations with known progress, use a ProgressBar instead (future component).

Spinner is the only component in the system that uses `linear` easing — all other animations use `ease-out`, `ease-in`, or `ease-in-out`. The constant rotation speed is intentional: uniform mechanical motion signals "process running" without implying acceleration or deceleration.

---

## The three ownership boundaries

**The token system decides:** nothing beyond font inheritance. Spinner deliberately reads no color token — it inherits `currentColor` from its parent, making it context-aware by default.

**The Spinner decides:** rotation speed (700ms, always), stroke width (2px always), animation easing (linear, always). These are not configurable.

**The consumer decides:** `size` and optionally `color` via the parent element's `color` CSS property. There is no `color` prop — color is a concern of the layout context, not the component.

---

## Sizes

Three sizes. Unlike Button and Input, Spinner heights are not derived from the typography system — they are independent pixel values based on visual balance at different scales.

| Size | Diameter | Stroke | Use |
| --- | --- | --- | --- |
| `sm` | 13px | 2px | Inside buttons (sm and md), inline loading states |
| `md` | 16px | 2px | Default. Standalone loading indicators, inside large buttons |
| `lg` | 20px | 2px | Page-level or panel-level loading states |

Stroke width is always 2px across all sizes. Thinner strokes (1px) look too fragile at 13px; thicker strokes (3px) look too heavy at 20px. 2px is the fixed value.

**Mapping to Button sizes:** Button uses `sm` spinner for `size="sm"` and `size="md"`, and `md` spinner for `size="lg"`. This is specified in the Button spec and is a decision of the Button component — Spinner does not know which button contains it.

---

## Color inheritance

Spinner uses `currentColor` for both the visible arc and the transparent gap. This means it automatically adapts to its context:

```
Inside a primary button:    currentColor = --color-primary-foreground  (white)
Inside a danger button:     currentColor = --color-danger-foreground   (white)
Inside a ghost button:      currentColor = --color-text-secondary      (gray)
Standalone on a page:       currentColor = --color-text-secondary      (gray)
```

To change the spinner color, set `color` on the parent element — not via a prop. This is the "no color prop" rule applied to Spinner specifically.

```css
/* Correct — parent sets color context */
.loading-indicator {
  color: var(--color-text-tertiary);
}

/* Wrong — no color prop exists */
<Spinner color="gray" />
```

---

## Animation

```css
@keyframes spinner-rotate {
  to { transform: rotate(360deg); }
}

.spinner {
  animation: spinner-rotate 700ms linear infinite;
}
```

**700ms** — not 1000ms (too slow, feels stuck), not 500ms (too frantic). 700ms is the perceptual sweet spot: fast enough to feel responsive, slow enough to not create anxiety.

**`linear`** — the only permitted use of linear easing in the system. All other UI animations use ease-out, ease-in, or ease-in-out because linear motion looks mechanical. For a spinner, mechanical is correct — the constant speed communicates that a uniform process is running.

**`infinite`** — spinner runs until the parent component removes it from the DOM. There is no built-in exit animation. When the operation completes, the component that owns the spinner (Button, FormField, etc.) replaces the spinner with final content. The transition is the parent component's responsibility.

---

## Implementation technique

Spinner is implemented as a circular `<div>` (or SVG circle) with a partial border, not as an SVG path animation. The border technique is simpler, GPU-composited, and produces identical results:

```css
.spinner {
  border-radius: 50%;
  border: 2px solid currentColor;
  border-top-color: transparent;   /* creates the gap */
  animation: spinner-rotate 700ms linear infinite;
  flex-shrink: 0;
}

.spinner--sm { width: 13px; height: 13px; }
.spinner--md { width: 16px; height: 16px; }
.spinner--lg { width: 20px; height: 20px; }
```

The `border-top-color: transparent` creates the visual arc. Three-quarters of the circle is visible; one-quarter is transparent. As the element rotates, the gap appears to chase the arc — creating the spinner illusion.

SVG implementation is also acceptable. If using SVG, use a `stroke-dasharray` / `stroke-dashoffset` approach with the same 700ms linear animation on `transform: rotate`.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `size` | `"sm" / "md" / "lg"` | `"md"` | Diameter: 13px / 16px / 20px. Stroke is always 2px. |
| `label` | `string` | — | Accessible label for screen readers. Rendered as `aria-label` on the spinner element. If omitted, a parent element's `aria-live` region must announce the loading state instead. |
| `class` | `string` | — | Layout overrides only: margin, flex-shrink, align-self. |

---

## Token mapping

Spinner reads no color tokens. It reads one motion value:

```
Animation:   700ms linear infinite — written inline, no token
Color:       currentColor — inherited from parent CSS context
```

The absence of token consumption is intentional. Spinner is a low-level primitive that defers all color decisions to its container.

---

## Accessibility

- **`role="status"`** on the spinner root. Announces to screen readers that a loading state is active.
- **`aria-label`** via the `label` prop. If the spinner is standalone (not inside a Button with visible label), provide a label: `label="Loading entries"`. If the spinner is inside a Button that already has a label ("Saving..."), `aria-hidden="true"` on the spinner is appropriate — the button's label already describes the state.
- **`prefers-reduced-motion`:** The spinner stops animating — it renders as a static partial arc. The loading state is still communicated visually (the arc shape) and semantically (`role="status"`), without triggering motion sickness.

```css
@media (prefers-reduced-motion: reduce) {
  .spinner { animation: none; }
}
```

---

## Anti-patterns

**Using Spinner for determinate progress.** If the operation has a known completion percentage, use a ProgressBar. Spinner implies indeterminate duration — using it when duration is known mismatches user expectations.

**Multiple spinners on the same surface.** Two spinners side by side create visual competition. If multiple operations are running simultaneously, show one spinner with a label that covers both ("Loading vault...") or use a page-level loading state.

**Spinner without removing previous content.** The spinner replaces the content it is loading — it does not overlay it. A spinner floating above partially-loaded content creates a confused layout. Button uses the loading state correctly: it replaces the label with a spinner in-place.

**Custom animation speed or easing.** 700ms linear is not a preference — it is the system value. A spinner at 400ms feels urgent and anxious; at 1200ms it feels broken. Do not override via props or CSS.

**Spinner as permanent decoration.** Spinner must disappear when the operation completes. A spinner that stays visible after loading is done is a bug.