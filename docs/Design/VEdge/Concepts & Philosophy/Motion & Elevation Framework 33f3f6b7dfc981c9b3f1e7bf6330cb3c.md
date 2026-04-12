# Motion & Elevation Framework

### A principled system for animation, transition, and depth in desktop UI

---

## Part 1 — Motion

### The core principle: motion communicates, not decorates

Every animation must answer: what does this tell the user? Three legitimate reasons to animate:

- **Spatial orientation** — where did this element come from?
- **State change** — something changed; confirm it happened
- **Causality** — this happened because of that

### Desktop-specific constraints

Desktop motion must be faster than mobile. A cursor arrives at a target instantly — no need for the UI to "follow" the user's hand. **Target: 100–200ms for most interactions. Nothing above 400ms except deliberate confirmations.**

### Duration tokens

| Token | Value | Use |
| --- | --- | --- |
| `--duration-instant` | 0ms | Focus ring, disabled state — no animation |
| `--duration-micro` | 80ms | Active press/release |
| `--duration-fast` | 150ms | Hover, tooltip, dropdown — **default** |
| `--duration-base` | 200ms | Panel entrance, modal open |
| `--duration-slow` | 300ms | Toast notification |
| `--duration-slower` | 500ms | Onboarding — use sparingly |

### Easing tokens

| Token | Curve | Use |
| --- | --- | --- |
| `--ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | **Entrances** — elements appearing |
| `--ease-in` | `cubic-bezier(0.55, 0, 1, 0.45)` | **Exits** — elements disappearing |
| `--ease-in-out` | `cubic-bezier(0.45, 0, 0.55, 1)` | **Movement** — element A to B within viewport |
| `--ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | **Micro-interactions** — copy checkmark, toggle |

The entrance/exit pairing matters. Opening with `ease-out` and closing with `ease-in` feels cohesive.

### What to animate

```
Safe (GPU-composited):
  opacity, transform (translate/scale/rotate)
  background-color, border-color, box-shadow, color

Never animate (causes layout recalculation):
  width, height, margin, padding
  top, left, right, bottom, font-size, border-width
```

To hide an element: `opacity: 0; pointer-events: none` — never `display: none` with a transition.

### Animation patterns

**Dropdown open:** `translateY(-6px) + opacity(0) → translateY(0) + opacity(1)` — 150ms ease-out

**Modal open:** `scale(0.96) + translateY(6px) + opacity(0) → scale(1) + translateY(0) + opacity(1)` — 200ms ease-out. Scrim: opacity(0) → opacity(0.5) at 150ms (slightly slower than modal).

**Detail panel slide-in:** `translateX(16px) + opacity(0) → translateX(0) + opacity(1)` — 200ms ease-out. 16px offset, not full width — the panel reveals itself from the edge, not flies in.

**Toast:** Enter 300ms ease-out (deliberate — has information to communicate). Exit 150ms ease-in (quick — don't linger).

**Copy confirmation:** Icon swap → spinner at 80ms, then checkmark with spring overshoot at 200ms. The spring "lands" — communicates completion with physicality.

**Button press:** `scale(0.97)` at 80ms ease-out. Releases at 80ms ease-out.

### State transitions that never animate

```
disabled     — instant. Animated disable feels reluctant.
focus ring   — instant. Delayed ring feels broken to keyboard users.
error state  — instant. User just submitted and needs immediate feedback.
```

### Reduced motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
```

Reduced motion ≠ no motion. Color transitions and instant state changes remain appropriate. The goal is avoiding vestibular triggers, not removing all feedback.

### What not to do

- Never use `linear` easing for UI transitions. Only acceptable for infinite loading spinners.
- Never chain animations (A finishes, then B starts). If two elements appear together, they animate simultaneously.
- Never animate to fill time. A slow operation shows a progress indicator, not a decorative loop.

---

## Part 2 — Elevation

### The problem with shadows

Traditional shadow-based elevation breaks in dark mode (dark shadows on dark surfaces are invisible) and fails with custom backgrounds (wrong shadow color). The modern approach uses **tonal color** instead.

Higher surfaces are lighter (in dark mode) or more differentiated (in light mode). Depth is communicated through color value, not drop shadow.

### Elevation levels

| Level | Token | Shadow | Use |
| --- | --- | --- | --- |
| 0 — base | `color.background` | none | Page background |
| 1 — raised | `color.surface.1` | none (tonal only) | Cards, sidebar, panels |
| 2 — overlay | `color.surface.2` | shadow-sm (optional) | Hover state, focused input |
| 3 — floating | `color.surface.4` | shadow-md | Dropdowns, tooltips, popovers |
| 4 — modal | `color.surface.4`  • scrim | shadow-lg | Dialogs, sheets |

Shadow is used conservatively — only at Level 3 and above. Levels 0–2 rely entirely on tonal color.

### Elevation and border

At Level 1, tonal difference alone may be subtle. A 0.5px border adds structure:

```css
/* card / panel */
background: var(--color-surface-1);
border: 0.5px solid var(--color-border);
border-radius: 8px;

/* popover / dropdown */
background: var(--color-surface-4);
border: 0.5px solid var(--color-border-strong);
box-shadow: var(--shadow-md);
border-radius: 8px;
```

Border defines the edge of a surface. Shadow communicates floating. They are not interchangeable.

---

## Part 3 — Token definitions and Tailwind v4 integration

### Motion tokens — canonical names

```css
/* in tokens.css → @theme inline */

--duration-instant:  0ms;
--duration-micro:   80ms;
--duration-fast:   150ms;
--duration-base:   200ms;
--duration-slow:   300ms;
--duration-slower: 500ms;

--ease-out:    cubic-bezier(0.16, 1, 0.3, 1);
--ease-in:     cubic-bezier(0.55, 0, 1, 0.45);
--ease-in-out: cubic-bezier(0.45, 0, 0.55, 1);
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
```

`--ease-linear` is intentionally omitted. Linear is only used for spinners (`animation: spin 700ms linear infinite`) and should be written inline in the component's CSS file.

### Tailwind motion utilities

```html
<!-- Single-property hover (Tailwind) -->
<div class="transition-colors duration-fast ease-out hover:bg-surface-2">
```

Use plain CSS for multi-property transitions:

```css
/* plain CSS — required for component state machines */
.btn {
  transition:
    background-color var(--duration-fast)  var(--ease-out),
    transform        var(--duration-micro) var(--ease-out);
}
```

### Shadow tokens — per-theme values

```css
/* :root (light preset) */
--shadow-none:  none;
--shadow-sm:    0 1px 3px rgba(0, 0, 0, 0.08);
--shadow-md:    0 4px 12px rgba(0, 0, 0, 0.12);
--shadow-lg:    0 8px 32px rgba(0, 0, 0, 0.16);
--shadow-scrim: rgba(0, 0, 0, 0.50);

/* [data-theme="dark"] injected by Rust */
--shadow-sm:    none;
--shadow-md:    0 0 0 1px rgba(255, 255, 255, 0.10);
--shadow-lg:    0 0 0 1px rgba(255, 255, 255, 0.14);
--shadow-scrim: rgba(0, 0, 0, 0.70);
```

Components reference shadow tokens by name. They never know whether the resolved value is a drop shadow or a ring:

```css
.dropdown {
  box-shadow: var(--shadow-md);   /* drop shadow in light, ring in dark */
}
```

### Elevation level reference

```css
/* Level 0 — base */
background: var(--color-background);

/* Level 1 — raised (card, sidebar) */
background: var(--color-surface-1);
border: 0.5px solid var(--color-border);

/* Level 2 — overlay (hover, focused input) */
background: var(--color-surface-2);
box-shadow: var(--shadow-sm);

/* Level 3 — floating (dropdown, tooltip) */
background: var(--color-surface-4);
border: 0.5px solid var(--color-border-strong);
box-shadow: var(--shadow-md);

/* Level 4 — modal */
background: var(--color-surface-4);
border: 0.5px solid var(--color-border-strong);
box-shadow: var(--shadow-lg);

/* Scrim */
background: var(--shadow-scrim);
```

---

## Summary of rules

1. Every animation must have a reason. If it does not communicate state, relationship, or causality — remove it.
2. Desktop animations target 100–200ms. Nothing above 400ms in daily-use UI.
3. Entrances use `ease-out`. Exits use `ease-in`. Movement within the viewport uses `ease-in-out`.
4. `ease-spring` is reserved for micro-interactions only. Never use it on layout-affecting properties.
5. Only animate `opacity`, `transform`, `background-color`, `border-color`, and `box-shadow`. Never animate layout properties.
6. Elevation is communicated by tonal color (surface layers), not shadow. Shadow is used only at Level 3 and above.
7. Border defines surface edges. Shadow communicates floating. They serve different purposes.
8. Reduced motion must be respected. Color transitions and instant state changes remain appropriate.
9. Vibrancy is a progressive enhancement. The tonal fallback must look correct independently.
10. When an element changes elevation, animate the transition using the same duration and easing as any other state change.