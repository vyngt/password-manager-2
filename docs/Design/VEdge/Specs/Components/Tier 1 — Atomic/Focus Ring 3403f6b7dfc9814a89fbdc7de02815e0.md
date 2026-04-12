# Focus Ring

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

The system-wide keyboard focus indicator. Every interactive element must display a visible focus ring when reached via keyboard navigation. Focus Ring is a **global CSS rule**, not a per-component opt-in. No component implements it independently — they inherit it.

---

## Specification

```css
/* Applied globally in tokens.css or base.css */

:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
  border-radius: inherit;
}

:focus:not(:focus-visible) {
  outline: none;
}
```

`outline` is used instead of `box-shadow` to avoid conflicts with elevation shadows on the same element. `border-radius: inherit` ensures the ring follows the component's shape (rounded button → rounded ring, pill badge → pill ring).

---

## Appearance

| Property | Value |
| --- | --- |
| Style | 2px solid |
| Color | `--color-focus-ring` (blue-600, #2563EB) |
| Offset | 2px |
| Timing | 0ms — instant, always |
| Shape | `border-radius: inherit` |
| WCAG contrast | 4.5:1 on white background — meets AA |

---

## The four non-negotiable rules

**Always instant.** Duration is always `--duration-instant` (0ms). A delayed ring feels broken — the user has already pressed Tab and needs immediate confirmation. This is defined in `component-system-rules.md` and cannot be changed per component.

**Never hidden.** No component may suppress `:focus-visible` without providing a visually equivalent replacement. `outline: none` on `:focus-visible` is an accessibility violation.

**`:focus-visible`, not `:focus`.** Mouse clicks must not show the ring. `:focus-visible` fires for keyboard navigation and programmatic focus from a keyboard context. Mouse-initiated `:focus` gets `outline: none`.

**Not clipped.** The ring must remain visible even inside `overflow: hidden` containers. If clipping occurs, use `overflow: visible` on the container or `outline-offset: -2px` as a last resort.

---

## Components that render a custom ring

Some components suppress the global rule and apply their own equivalent:

| Component | Custom ring behavior |
| --- | --- |
| Input | Focus ring replaces the default border: 2px solid `--color-focus-ring` on the full field boundary |
| Select | Same as Input |
| Textarea | Same as Input |
| SegmentedControl | Ring on outer container boundary, not on individual segments |

In all cases, the visual output must be perceptually identical to the global rule. These components set `outline: none` on `:focus-visible` for themselves and apply ring styles manually via class or attribute selectors.

---

## Token mapping

| Property | Token |
| --- | --- |
| Ring color | `--color-focus-ring` |
| Ring width | 2px (not a token — fixed value) |
| Ring offset | 2px (not a token — fixed value) |
| Ring timing | `--duration-instant` (0ms) |

---

## Accessibility

- WCAG 2.1 SC 2.4.7 Focus Visible — Level AA
- WCAG 2.1 SC 2.4.11 Focus Appearance (Enhanced) — Level AA
    - Minimum 2px outline ✓
    - Sufficient contrast against adjacent colors ✓ (4.5:1 blue on white)
- `prefers-reduced-motion` does not apply to the focus ring — it is already instant

---

## Do not use

- Do not add `outline: none` to any element without providing an equivalent visible alternative
- Do not reduce outline width below 2px
- Do not animate the ring appearance
- Do not change ring color per-component — `--color-focus-ring` is the only permitted value

---

## CSS implementation

Focus Ring is a **global rule**, not a component file. It lives in `base.css` or directly in `tokens.css`, applied once for the entire app.

```css
/* base.css — applied once globally */

:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
  border-radius: inherit;
}

:focus:not(:focus-visible) {
  outline: none;
}
```

Components that implement a custom ring suppress the global rule on themselves and replicate the appearance manually:

```css
/* input.css — custom ring = active border replacement */
.input:focus-within {
  outline: none;                          /* suppress global rule on container */
  border-color: var(--color-focus-ring);
  border-width: 2px;
}

/* segmented-control.css — ring on outer boundary, not on individual segments */
.segmented-control:focus-within {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}

.segmented-control__segment:focus-visible {
  outline: none; /* suppressed — parent handles the ring */
}
```

All custom ring implementations must produce a result visually identical to the global rule: 2px solid `--color-focus-ring`, 2px offset.

---

## Implementation checklist

- [ ]  Global rule in `base.css`: `:focus-visible` gets `outline: 2px solid var(--color-focus-ring)`, `outline-offset: 2px`, `border-radius: inherit`
- [ ]  `:focus:not(:focus-visible)` gets `outline: none` — mouse clicks show no ring
- [ ]  Input / Select / Textarea: suppress global rule, apply 2px border on `focus-within`
- [ ]  SegmentedControl: ring on outer container, suppress on individual segments
- [ ]  No component has `outline: none` on `:focus-visible` without an equivalent replacement
- [ ]  Ring not clipped by `overflow: hidden` in any container
- [ ]  Ring visible inside modal (above scrim)
- [ ]  Contrast ratio ≥ 4.5:1 for `--color-focus-ring` on all surfaces verified
- [ ]  `prefers-reduced-motion` does not affect the ring — it is already instant (0ms)