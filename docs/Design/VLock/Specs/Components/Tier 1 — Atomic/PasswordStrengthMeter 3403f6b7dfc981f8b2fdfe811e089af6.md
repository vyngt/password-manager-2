# PasswordStrengthMeter

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

Visual indicator of password entropy shown in real time as the user types. Communicates password quality across four levels without requiring the user to decode numbers or percentages.

PasswordStrengthMeter **does not compute entropy** — that is the consumer's responsibility. It only visualizes a score passed to it as a prop.

---

## The three ownership boundaries

**The token system decides:** segment colors per score level, border radius, transition duration for fill animation.

**PasswordStrengthMeter decides:** segment count (always 4), fill progression left-to-right, label text per score level, instant snap on score decrease.

**The consumer decides:** score (computed externally via zxcvbn or equivalent), whether to show the label.

---

## Score levels

| Score | Level | Filled segments | Color token | Label |
| --- | --- | --- | --- | --- |
| `0` | Empty | 0 | `--color-surface-3` (all) | — |
| `1` | Weak | 1 | `--color-danger` | "Weak" |
| `2` | Fair | 2 | `--color-warning` | "Fair" |
| `3` | Strong | 3 | `--color-primary` | "Strong" |
| `4` | Very strong | 4 | `--color-success` | "Very strong" |

Unfilled segments always render at `--color-surface-3`.

---

## Anatomy

```
[ seg 1 ] [ seg 2 ] [ seg 3 ] [ seg 4 ]
                                [ label ]
```

- 4 segments, 4px gap between each
- Total width: fills container (100%)
- Segment height: 4px fixed
- Label: `text-xs` (12px), right-aligned below segments, color matches the active segment token
- 4px gap between segments row and label

---

## Motion

| Direction | Behavior |
| --- | --- |
| Score increases | New segment animates in: width 0 → 100% over 150ms ease-out |
| Score decreases | Segments snap instantly — no reverse animation. Reversal is corrective, not decorative. |

Under `prefers-reduced-motion`: all transitions are 0ms.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `score` | `0 / 1 / 2 / 3 / 4` | `0` | Strength level from external library |
| `show_label` | `bool` | `true` | Show text label below segments. Only omit if an external label is present. |

---

## Token mapping

| Score | Segment color |
| --- | --- |
| 0 | `--color-surface-3` |
| 1 | `--color-danger` |
| 2 | `--color-warning` |
| 3 | `--color-primary` |
| 4 | `--color-success` |

---

## Accessibility

- `role="meter"` on the container
- `aria-valuenow={score}`, `aria-valuemin="0"`, `aria-valuemax="4"`
- `aria-label="Password strength"` on container
- `aria-valuetext` maps score to human label: "Weak" / "Fair" / "Strong" / "Very strong" / ""
- Label text is the primary accessible indicator — color alone is never the only signal
- Works correctly under `prefers-reduced-motion` — fill snaps, label updates normally

---

## Do not use

- To compute password strength (use zxcvbn or equivalent — pass the result as `score`)
- Outside of a password entry context
- With `show_label=false` unless an external label explicitly communicates the current level

---

## CSS implementation

```css
.strength-meter {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.strength-meter__segments {
  display: flex;
  gap: 4px;
}

.strength-meter__segment {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: var(--color-surface-3);
  overflow: hidden;
}

.strength-meter__fill {
  height: 100%;
  width: 0;
  border-radius: 2px;
  transition: width var(--duration-fast) var(--ease-out);
}

/* Score levels — fill left-to-right, snap on decrease */
.strength-meter--1 .strength-meter__segment:nth-child(1) .strength-meter__fill
{ width: 100%; background: var(--color-danger); }

.strength-meter--2 .strength-meter__segment:nth-child(-n+2) .strength-meter__fill
{ width: 100%; background: var(--color-warning); }

.strength-meter--3 .strength-meter__segment:nth-child(-n+3) .strength-meter__fill
{ width: 100%; background: var(--color-primary); }

.strength-meter--4 .strength-meter__segment .strength-meter__fill
{ width: 100%; background: var(--color-success); }

.strength-meter__label {
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  text-align: right;
  min-height: 16px;
}

.strength-meter--0 .strength-meter__label { color: transparent; }
.strength-meter--1 .strength-meter__label { color: var(--color-danger); }
.strength-meter--2 .strength-meter__label { color: var(--color-warning); }
.strength-meter--3 .strength-meter__label { color: var(--color-primary); }
.strength-meter--4 .strength-meter__label { color: var(--color-success); }

@media (prefers-reduced-motion: reduce) {
  .strength-meter__fill { transition-duration: 0.01ms !important; }
}
```

---

## Implementation checklist

- [ ]  `styles/components/strength-meter.css` created
- [ ]  4 segments render at all scores (0–4)
- [ ]  Unfilled segments show `--color-surface-3`
- [ ]  Score 1–4: correct color token per level (danger / warning / primary / success)
- [ ]  Fill animates on score increase: 150ms ease-out
- [ ]  Fill snaps instantly on score decrease — no reverse animation
- [ ]  Label text matches score level, color matches active segment
- [ ]  `show_label=false` hides label correctly
- [ ]  `role="meter"` + `aria-valuenow` / `aria-valuemin` / `aria-valuemax` / `aria-valuetext` set
- [ ]  `prefers-reduced-motion`: fill snaps, label updates normally
- [ ]  No hardcoded values — every color reads a token