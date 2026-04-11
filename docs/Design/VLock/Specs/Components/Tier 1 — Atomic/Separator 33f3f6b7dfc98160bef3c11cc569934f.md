# Separator

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A visual divider that creates a clear boundary between two sections or groups of content. Separator communicates spatial organization without adding any semantic or interactive meaning — it is structural chrome, not content.

---

## The three ownership boundaries

**The token system decides:** color (`--color-border`), thickness (1px), and the typography tokens used in the labeled variant.

**The Separator decides:** how the labeled variant centers text between two lines, and how vertical separators stretch to fill their container height.

**The consumer decides:** orientation (`horizontal` / `vertical`) and optional label text for section dividers. Separator has no variant, no size, no state.

---

## Orientations

**`horizontal`** (default) — A 1px horizontal line spanning 100% of its container width. Used between sections in forms, settings panels, and card bodies.

**`vertical`** — A 1px vertical line stretching to the height of its flex container. Used between inline button groups, toolbar items, and sidebar item groups.

```
horizontal:  ─────────────────────

vertical:    │   (height from parent flex context)

labeled:     ─────── SECTION ───────
```

---

## Labeled variant

When the `label` prop is provided, the separator renders overline text centered between two lines. The label communicates the relationship of the sections on either side.

```
label typography: role.overline
  font-size:      text-xs (12px)
  font-weight:    500 (medium)
  text-transform: uppercase
  letter-spacing: 0.06em (tracking-wide)
  color:          --color-text-tertiary

label padding:    0 12px from each line
line height:      1px, --color-border, flex: 1
```

Labeled separator is horizontal only. A vertical separator with a label is not a valid pattern.

Label text should be 1–3 words. "General", "Advanced", "Danger Zone". Never a sentence.

---

## Thickness and color

All separators use a single fixed value:

```
thickness: 1px
color:     var(--color-border)      default
           var(--color-border-strong)  if stronger emphasis needed
```

`--color-border` is the correct default. `--color-border-strong` is only for separators that must compete with adjacent surface tones that make the default border invisible (e.g., inside a surface-3 or surface-4 background). This is rare — use it intentionally, not by default.

---

## Spacing

Separator does not own its surrounding spacing. The parent layout provides margins. This keeps the component's responsibility minimal and consistent.

```css
/* Correct — parent provides spacing */
.settings-section + .settings-section {
  margin-top: 32px;
}
.settings-section > .separator {
  margin: 24px 0;
}

/* Wrong — separator managing its own position */
<Separator margin="24px 0" />
```

Separator accepts a `class` prop for layout-only overrides (margin). It does not have margin props.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `orientation` | `"horizontal" / "vertical"` | `"horizontal"` | Axis of the divider line. |
| `label` | `string` | — | Optional section label centered in the line. Horizontal only. Plain string, rendered in overline role. |
| `strong` | `boolean` | `false` | Uses `--color-border-strong` instead of `--color-border`. Use only when default border is not visible against the surface. |
| `class` | `string` | — | Layout overrides only: margin. |

---

## Token mapping

```
--color-border          default line color
--color-border-strong   strong variant line color

Labeled variant typography:
--text-xs / --text-xs--line-height
--font-weight-medium    500
--tracking-wide         0.06em
--color-text-tertiary
```

---

## CSS implementation reference

```css
/* Horizontal */
.separator--horizontal {
  display: block;
  width: 100%;
  height: 1px;
  background: var(--color-border);
  border: none;
  flex-shrink: 0;
}

.separator--horizontal.separator--strong {
  background: var(--color-border-strong);
}

/* Vertical */
.separator--vertical {
  display: block;
  width: 1px;
  height: auto;         /* stretches in flex context */
  align-self: stretch;  /* required: parent must be display:flex */
  background: var(--color-border);
  flex-shrink: 0;
}

/* Labeled (horizontal only) */
.separator--labeled {
  display: flex;
  align-items: center;
  gap: 12px;
}

.separator--labeled::before,
.separator--labeled::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--color-border);
}

.separator__label {
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  font-weight: var(--font-weight-medium);
  letter-spacing: var(--tracking-wide);
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  white-space: nowrap;
  user-select: none;
}
```

### Semantic HTML

Use `<hr>` for horizontal separators, not `<div>`. The `<hr>` element has built-in `role="separator"` and `aria-orientation="horizontal"` semantics. Reset its browser default styles:

```css
hr.separator--horizontal {
  margin: 0;
  border: none;
  background: var(--color-border);
  height: 1px;
}
```

For vertical separators and labeled separators, use `<div role="separator" aria-orientation="vertical">` since `<hr>` cannot be styled as vertical reliably across browsers.

---

## Anti-patterns

**Using separator to add spacing.** Separator communicates structure, not space. If sections feel too close together, the parent layout needs more margin — not a separator.

**Separator before every section.** Separators between every group create visual noise that makes nothing feel separate. Use whitespace (gap, margin) as the primary grouping mechanism; use separator only when whitespace alone is insufficient.

**Long labels.** "Danger Zone" is correct. "Advanced settings that may cause data loss" is not — that belongs in descriptive text, not a separator label.

**Vertical separator outside a flex context.** `align-self: stretch` only works when the separator's parent is `display: flex`. A vertical separator inside a block layout collapses to 0 height.