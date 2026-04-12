# HelperText

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

Supplementary feedback text rendered below a form field. Communicates validation state, format hints, or constraints. Always paired with an Input, Select, or Textarea via `aria-describedby` — never floating free.

---

## The three ownership boundaries

**The token system decides:** text color per status, font size (always `text-xs` / 12px), icon size.

**HelperText decides:** which icon to render per status, `role="alert"` on error.

**The consumer decides:** status, message content, `id` (required for `aria-describedby`).

---

## Variants

| Status | Color | Icon | `role` | When to use |
| --- | --- | --- | --- | --- |
| `hint` | `--color-text-secondary` | None | — | Format tips, character limits, contextual guidance |
| `error` | `--color-danger-text` | `AlertCircle` 12px | `alert` | Validation failed — value is wrong or missing |
| `success` | `--color-success-text` | `CheckCircle` 12px | — | Validation passed ("Username available") |
| `warning` | `--color-warning-text` | `AlertTriangle` 12px | — | Valid but needs attention |

---

## Anatomy

```
[ icon (12px, optional) ] [ message text ]
```

- Font: `text-xs` (12px / 16px line-height), always
- Gap between icon and text: 4px
- Spacing above: 6px from the bottom of the parent field
- Left-aligned
- Maximum 2 lines. Truncate with ellipsis beyond that.

---

## States

No interactive states. HelperText is purely presentational.

Color change (e.g., hint → error on submit) is **instant** — 0ms, no transition. Validation feedback must not feel delayed.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `status` | `"hint" / "error" / "success" / "warning"` | `"hint"` | Drives color and icon |
| `message` | `string` | — | The feedback text |
| `id` | `string` | — | Required. Must match the parent field's `aria-describedby` |

---

## Token mapping

| Status | Text color | Icon color |
| --- | --- | --- |
| `hint` | `--color-text-secondary` | — |
| `error` | `--color-danger-text` | `--color-danger-text` |
| `success` | `--color-success-text` | `--color-success-text` |
| `warning` | `--color-warning-text` | `--color-warning-text` |

---

## Accessibility

- `role="alert"` on error status — causes screen readers to announce immediately on state change
- `id` is required and must be referenced by the parent field's `aria-describedby`
- Color alone does not communicate status — the icon prefix is required for all non-hint statuses
- Screen reader reads: icon label + message text. Icon `aria-label`: "Error:", "Success:", "Warning:"

---

## Do not use

- Without a paired form field (`aria-describedby` connection is required — floating HelperText is not accessible)
- For long explanations over 2 lines — use a Tooltip or a dedicated help panel
- More than one HelperText per field — combine multiple messages into one string

---

## CSS implementation

```css
.helper-text {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  margin-top: var(--space-1_5);
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  line-height: 16px;
}

/* Color is instant — no transition on state change */
.helper-text--hint    { color: var(--color-text-secondary); }
.helper-text--error   { color: var(--color-danger-text); }
.helper-text--success { color: var(--color-success-text); }
.helper-text--warning { color: var(--color-warning-text); }

.helper-text__icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
}
```

---

## Implementation checklist

- [ ]  `styles/components/helper-text.css` created
- [ ]  All 4 statuses render correct color: hint / error / success / warning
- [ ]  Icon renders at 12px for error / success / warning; absent for hint
- [ ]  `role="alert"` applied on error status only
- [ ]  `id` prop wired — parent field references it via `aria-describedby`
- [ ]  Color change is instant (0ms) — no transition
- [ ]  `margin-top: var(--space-1_5)` spacing above
- [ ]  No hardcoded values — every color reads a token