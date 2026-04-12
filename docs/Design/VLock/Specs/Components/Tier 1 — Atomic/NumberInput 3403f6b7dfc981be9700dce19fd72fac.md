# NumberInput

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

Numeric input with custom increment/decrement stepper controls. Does **not** use `type="number"` — the browser's native spin buttons are not styleable cross-platform. NumberInput renders its own ± steppers and enforces `min`/`max`/`step` in JavaScript.

Use wherever a constrained numeric value is needed: port numbers, timeout durations, key lengths, iteration counts, password length settings.

---

## The three ownership boundaries

**The token system decides:** border color per state, height per size, focus ring appearance, transition durations and easing.

**NumberInput decides:** stepper button layout, min/max enforcement (clamp on blur), step enforcement, overflow behavior (steppers disable at boundary).

**The consumer decides:** value, min, max, step, unit label, size, disabled, status.

---

## Anatomy

Four zones left to right:

```
[ prefix / leadingIcon ] [ numeric text ] [ − ] [ + ]
```

The stepper buttons (28×28 at sm, 32×32 at md/lg) always appear in the trailing area. They cannot be removed — if the consumer does not want steppers, they should use `Input` with `inputmode="numeric"` instead.

An optional `unit` string renders as an inline suffix before the steppers: "px", "ms", "%", "bits".

---

## Sizes

Same height tokens as Button and Input — derived from the typography system.

| Size | Height | Font | Use |
| --- | --- | --- | --- |
| `sm` | 28px | 12px | Compact settings rows, inline table edits |
| `md` | 36px | 14px | Default. General use. |
| `lg` | 44px | 16px | Prominent standalone controls |

---

## States

Default, hover, focus, error, disabled. No loading state — NumberInput is synchronous.

The stepper buttons inherit the field's disabled state. Additionally: decrement is individually disabled when `value === min`; increment is individually disabled when `value === max`. Individual stepper disabling does not remove the field's focus or tab stop.

---

## Keyboard

| Key | Behavior |
| --- | --- |
| `↑` / `↓` | Increment / decrement by `step` |
| `Page Up` / `Page Down` | Increment / decrement by `10 × step` |
| `Home` | Jump to `min` |
| `End` | Jump to `max` |
| Direct typing | Accepts digits and `-`. Non-numeric input ignored. Value clamped to min/max on blur. |

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `value` | `f64` | — | Controlled value |
| `default_value` | `f64` | `0` | Uncontrolled default |
| `min` | `f64` | `f64::NEG_INFINITY` | Minimum allowed value |
| `max` | `f64` | `f64::INFINITY` | Maximum allowed value |
| `step` | `f64` | `1` | Increment / decrement amount |
| `unit` | `string` | — | Suffix label rendered before steppers: "px", "ms", "%" |
| `size` | `"sm" / "md" / "lg"` | `"md"` | Physical scale |
| `status` | `"default" / "error" / "success" / "warning"` | `"default"` | Validation state. Drives border color. |
| `disabled` | `bool` | `false` | Inerts entire component including steppers |
| `on_change` | `Callback<f64>` | — | Fires on every committed value change (blur or stepper click) |

---

## Token mapping

| Element | Token |
| --- | --- |
| Border default | `--color-border` |
| Border focus | `--color-focus-ring` |
| Border error | `--color-danger` |
| Border success | `--color-success` |
| Background | `--color-background` |
| Text | `--color-text-primary` |
| Unit / suffix text | `--color-text-tertiary` |
| Stepper icon | `--color-text-secondary` |
| Stepper hover bg | `--color-surface-2` |
| Stepper disabled | `opacity: 0.4` |
| Height (sm/md/lg) | `--height-sm` / `--height-md` / `--height-lg` |

---

## Accessibility

- Uses `<input type="text" inputmode="numeric" pattern="[0-9]*">` — avoids browser spin buttons while retaining mobile numeric keyboard
- `role="spinbutton"` on the input element
- `aria-valuemin`, `aria-valuemax`, `aria-valuenow` reflect current min/max/value
- Decrement: `aria-label="Decrease"`, `aria-disabled="true"` at min
- Increment: `aria-label="Increase"`, `aria-disabled="true"` at max
- Connected to Label via `id` / `aria-labelledby`

---

## Do not use

- For free-form numeric text with no constraints (use `Input` with `inputmode="numeric"`)
- For selecting from a discrete list of values (use `Select`)
- For a percentage with a visual track (use `Slider`)
- For currency — formatting and locale rules are outside this component's scope

---

## CSS implementation

```css
.number-input {
  display: flex;
  align-items: center;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-background);
  transition: border-color var(--duration-fast) var(--ease-out);
  overflow: hidden;
}

.number-input:focus-within {
  border: 2px solid var(--color-focus-ring);
}

.number-input--sm { height: var(--height-sm); }
.number-input--md { height: var(--height-md); }
.number-input--lg { height: var(--height-lg); }

.number-input__field {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  color: var(--color-text-primary);
  font-family: var(--font-sans);
  padding: 0 var(--space-3);
}

.number-input--sm .number-input__field { font-size: var(--text-xs); }
.number-input--md .number-input__field { font-size: var(--text-sm); }
.number-input--lg .number-input__field { font-size: var(--text-base); }

.number-input__unit {
  color: var(--color-text-tertiary);
  font-size: var(--text-sm);
  padding-right: var(--space-2);
  user-select: none;
}

.number-input__steppers {
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--color-border);
}

.number-input__step {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  flex: 1;
  cursor: pointer;
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  outline: none;
  transition: background-color var(--duration-fast) var(--ease-out);
}

.number-input__step--decrement {
  border-top: 1px solid var(--color-border);
}

.number-input__step:hover:not(:disabled) {
  background: var(--color-surface-2);
  color: var(--color-text-primary);
}

.number-input__step:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: -2px;
}

.number-input__step:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Status */
.number-input--error   { border-color: var(--color-danger); }
.number-input--success { border-color: var(--color-success); }
.number-input--warning { border-color: var(--color-warning); }

/* Disabled */
.number-input--disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

@media (prefers-reduced-motion: reduce) {
  .number-input,
  .number-input__step { transition-duration: 0.01ms !important; }
}
```

---

## Implementation checklist

- [ ]  `styles/components/number-input.css` created
- [ ]  Stepper buttons (− / +) render at correct size for sm / md / lg
- [ ]  Decrement disabled at `min`, increment disabled at `max`
- [ ]  Keyboard: `↑` / `↓` step, `Page Up` / `Page Down` ×10, `Home` / `End` clamp to min/max
- [ ]  Direct typing: non-numeric chars ignored, value clamped on blur
- [ ]  Status border colors: error / success / warning
- [ ]  `focus-within` ring: 2px solid `--color-focus-ring`, instant (0ms)
- [ ]  Disabled: `opacity: 0.4`, `pointer-events: none` on root
- [ ]  Unit label renders in `--color-text-tertiary`
- [ ]  `role="spinbutton"` + `aria-valuemin` / `aria-valuemax` / `aria-valuenow` set
- [ ]  Stepper `aria-label` and `aria-disabled` correct at boundary
- [ ]  No hardcoded values — every color, size, duration reads a token
- [ ]  `prefers-reduced-motion` handled