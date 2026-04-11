# ColorPicker

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A structured color selection UI for choosing a color value with precision. ColorPicker replaces `<input type="color">` entirely — the browser native color picker is platform-specific, cannot adopt token styling, and looks different on macOS, Windows, and Linux.

ColorPicker is a Tier 1 component despite its internal complexity because it exposes a single, atomic interface to its consumer: one color value in, one color value out. All internal layout, interaction, and conversion logic is owned by the component.

---

## The three ownership boundaries

**The token system decides:** panel elevation (Level 3), border, shadow, border radius, typography for inputs, and all focus ring behavior. The color gradient and hue/alpha slider tracks are rendered values, not token values — they represent the actual color space.

**The ColorPicker decides:** internal color representation (HSV for lossless manipulation), format conversion between hex/rgb/hsl on output, gradient thumb positioning from HSV state, hue and alpha slider behavior, eyedropper availability detection, and all keyboard interaction within the panel.

**The consumer decides:** `value` (current color string), `format` (output format), `alpha` (whether opacity is enabled), `swatches` (preset colors), `disabled`, and `onChange`. The consumer always works with color strings — never with raw HSV or component-internal state.

---

## Anatomy

```
[trigger — color swatch + value text or swatch-only]

[panel — Level 3 elevation, portal]
  [gradient-area]         ← 2D saturation/brightness picker
    [gradient-thumb]      ← draggable circle
  [hue-slider]            ← horizontal rainbow strip
  [alpha-slider?]         ← shown only when alpha=true
  [── divider ──]
  [format-row]
    [eyedropper-btn?]     ← shown when EyeDropper API available
    [format-switcher]     ← HEX / RGB / HSL tabs
    [color-inputs]        ← editable fields for current format
  [swatches?]             ← shown when swatches prop provided
    [swatch × n]          ← preset color squares
```

---

## Internal color model

All internal state is stored as **HSV (Hue, Saturation, Value)** plus optional **Alpha**. HSV is lossless for picker manipulation — converting hex → HSV → hex preserves the original value exactly, whereas hex → HSL → hex can drift due to floating-point rounding.

```
HSV:
  H: 0–360°  (hue — maps to the hue slider position)
  S: 0–100%  (saturation — maps to the X axis of the gradient area)
  V: 0–100%  (value/brightness — maps to the Y axis, 100% at top)
  A: 0–100%  (alpha — maps to the alpha slider, always 100% if alpha=false)
```

On every change, HSV is converted to the consumer's chosen output format before `onChange` fires. The consumer never sees HSV.

---

## Trigger

The trigger is a button that opens the color panel. Two modes controlled by the `triggerMode` prop:

**`swatch-input`** (default) — A color swatch square on the left and the current hex value as editable text on the right. Looks like an Input with a leading color square. Width follows parent layout.

```
[■ swatch 20×20] [ #2563EB            ▾ ]
```

**`swatch-only`** — A square button showing only the color swatch. Fixed 36×36px (md) or 28×28px (sm). Use when space is constrained and the hex value is shown elsewhere.

```
[■ 36×36]
```

Both modes show a dropdown chevron on hover to communicate that clicking opens a panel. The trigger's border and focus ring follow the same tokens as Input — `--color-border`, `--color-primary` focus, `--color-focus-ring` ring.

---

## Panel

**Elevation:** Level 3 — `--color-surface-4`, `0.5px --color-border-strong`, `--shadow-md`.

**Width:** 240px fixed. Neither wider nor narrower — this is the canonical size that fits the gradient area, sliders, and inputs without waste.

**Position:** Below the trigger by default. Same placement rules as Select dropdown: preferred side, auto-flip if viewport clearance < 8px, 8px offset from trigger.

**Panel padding:** 12px.

### Gradient area

A 216×136px interactive region (width = panel width minus 2×12px padding).

Background is a layered CSS gradient:

```css
.gradient-area {
  background:
    linear-gradient(to bottom, transparent, #000),   /* value: 100% top, 0% bottom */
    linear-gradient(to right, #fff, hsl(var(--hue), 100%, 50%));  /* saturation */
}
```

The hue variable `--hue` is derived from the current H value and updated as the hue slider changes.

**Thumb:** 12×12px circle. Border: 2px solid white. Box-shadow: `0 0 0 1px rgba(0,0,0,0.25)` (so it's visible on both light and dark gradient areas). Positioned absolutely based on S (X) and V (Y) values:

```
left  = S% × gradient-width
top   = (100 - V%) × gradient-height
```

**Interaction:** Click anywhere in the gradient area to jump the thumb. Drag to update continuously. Thumb follows cursor during drag, clamped to area bounds.

**Keyboard (when gradient area is focused):**

| Key | Action |
| --- | --- |
| ← / → | Saturation −1% / +1% |
| ↑ / ↓ | Value +1% / −1% |
| Shift + ← / → | Saturation −10% / +10% |
| Shift + ↑ / ↓ | Value +10% / −10% |

### Hue slider

A horizontal `<input type="range">` (0–360), heavily CSS-styled to look like a rainbow strip:

```css
.hue-slider::-webkit-slider-runnable-track {
  background: linear-gradient(to right,
    hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%),
    hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), hsl(360,100%,50%));
  border-radius: var(--radius-full);
  height: 10px;
}
```

Thumb: 16×16px white circle with same shadow as gradient thumb. Height of the slider track: 10px.

### Alpha slider

Shown only when `alpha=true`. Same dimensions and styling as hue slider but with a different track:

```css
.alpha-slider::-webkit-slider-runnable-track {
  background:
    linear-gradient(to right, transparent, hsl(var(--hue), var(--sat)%, var(--lit)%)),
    url("checkerboard.svg");  /* shows through transparent portion */
}
```

The checkerboard pattern underneath communicates transparency visually. 4×4px squares, alternating `#ccc` and `#fff`.

### Format row

A compact row below the sliders:

```
[eyedropper?] [ HEX | RGB | HSL ] [ input fields ]
```

**Eyedropper button:** Icon-only 28×28px ghost button. Shown only when `window.EyeDropper` is defined (progressive enhancement). On click, opens native OS eyedropper. `aria-label="Pick color from screen"`.

**Format switcher:** A mini SegmentedControl-like control (not the full Tier 1 component — a simplified internal version). Switches between HEX, RGB, HSL. The active format drives what inputs are shown.

**Color inputs by format:**

```
HEX:  [ #________ ]   ← single 7-char input (9-char with alpha)
RGB:  [ R:___ ] [ G:___ ] [ B:___ ] [ A:___ ]?
HSL:  [ H:___ ] [ S:___ ] [ L:___ ] [ A:___ ]?
```

All inputs are mini — height 24px, font `text-xs`. They are the only interactive elements in the panel that accept keyboard text input.

Editing a hex input: the value is parsed and applied on blur or Enter. Invalid hex shows `status="error"` border on the input without closing the panel.

### Preset swatches

Optional. Shown when `swatches` prop is a non-empty array. A wrapping grid of 20×20px color squares, `gap: 4px`, max 2 rows before scrolling.

Each swatch: a square button with `border-radius: var(--radius-sm)`. Clicking applies that color. Focused swatch shows a 2px `--color-focus-ring` outline. `aria-label` = the color value (or a consumer-provided label).

---

## Sizes

ColorPicker has `sm` and `md` trigger sizes. The panel is always 240px wide — size only affects the trigger.

| Size | Trigger height | Swatch-only | Font |
| --- | --- | --- | --- |
| `sm` | 28px | 28×28px | `text-xs` 12px |
| `md` | 36px | 36×36px | `text-sm` 14px |

---

## Color format output

The `format` prop controls what string the consumer receives in `onChange`.

| Format | Example (no alpha) | Example (with alpha) |
| --- | --- | --- |
| `"hex"` (default) | `#2563EB` | `#2563EBCC` |
| `"rgb"` | `rgb(37, 99, 235)` | `rgba(37, 99, 235, 0.8)` |
| `"hsl"` | `hsl(221, 83%, 53%)` | `hsla(221, 83%, 53%, 0.8)` |

When `alpha=false` (default), alpha is always 1 / 100% and not included in the output string. When `alpha=true`, alpha is included when it is less than 100%.

Format conversion is always done via the internal HSV model — never by parsing the output string of one format and converting to another. This prevents rounding drift.

---

## States

**Closed (default):** Trigger shows current color swatch + value. Standard Input-like styling.

**Open:** Panel renders in portal at Level 3 elevation. Trigger shows `--color-primary` border + focus ring (same as Input focused state).

**Disabled:** `opacity: 0.4` on trigger. `pointer-events: none`. Panel cannot open.

**Invalid input:** When user types an invalid hex string in the format input, the input shows `status="error"` border. The panel does not close. The internal color state does not update until a valid value is entered.

---

## Motion

Panel open/close follows the same pattern as Select dropdown:

```
open:  translateY(-4px) + opacity(0) → translateY(0) + opacity(1)
       duration: var(--duration-fast) 150ms, ease-out

close: opacity(1) → opacity(0)
       duration: 100ms, ease-in
```

Gradient thumb drag: no transition — position updates synchronously with cursor. Any animation lag on drag feels broken.

Swatch color preview in trigger: transitions background-color at `duration-fast` (150ms) `ease-out` as the user drags.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `value` | `string` | `"#000000"` | Controlled color value. Must be a valid CSS color string in the format specified by `format`. |
| `defaultValue` | `string` | `"#000000"` | Uncontrolled default. |
| `format` | `"hex" / "rgb" / "hsl"` | `"hex"` | Output format for `onChange` and displayed in the format input. Internal state is always HSV. |
| `alpha` | `boolean` | `false` | Enables the alpha slider and includes opacity in the output value. |
| `swatches` | `{ value: string, label?: string }[]` | `[]` | Preset color swatches. Renders a swatch grid below the format row. Empty array hides the swatch section. |
| `triggerMode` | `"swatch-input" / "swatch-only"` | `"swatch-input"` | Trigger display mode. `swatch-only` is a square button showing just the color preview. |
| `size` | `"sm" / "md"` | `"md"` | Physical scale of the trigger. Panel is always 240px. |
| `disabled` | `boolean` | `false` | Opacity 0.4, pointer-events none. Panel cannot open. |
| `onChange` | `(value: string) => void` | — | Fires on every color change (drag, slider, input, swatch click). Receives color string in the selected `format`. |
| `onChangeEnd` | `(value: string) => void` | — | Fires only when interaction ends (mouseup, blur). Use for expensive operations like API calls that should not fire on every drag frame. |
| `class` | `string` | — | Layout overrides only: width (for swatch-input trigger), margin. |

---

## Token mapping

### Panel elevation tokens

```
--color-surface-4       panel background
--color-border-strong   0.5px panel border
--shadow-md             panel drop shadow
--radius-lg             panel border radius (8px)
```

### Trigger tokens (follows Input)

```
--color-surface-1       trigger background
--color-border          trigger border (resting)
--color-border-strong   trigger border (hover)
--color-primary         trigger border (open/focus)
--color-focus-ring      2px outline on focus
--color-text-primary    hex value text
--radius                trigger border radius (6px)
```

### Format input tokens

```
--color-surface-2       mini input background
--color-border          mini input border
--color-text-primary    input text
--text-xs               12px
--font-weight-normal    400
```

### Motion tokens

```
--duration-fast   150ms   panel enter
--duration-fast   150ms   trigger color preview transition
--ease-out              panel enter easing
--ease-in               panel exit easing
```

---

## CSS implementation reference

```css
/* Panel */
.color-picker-panel {
  position: absolute;
  z-index: 50;
  width: 240px;
  padding: 12px;
  background: var(--color-surface-4);
  border: 0.5px solid var(--color-border-strong);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* Gradient area */
.cp-gradient {
  width: 100%;
  height: 136px;
  border-radius: var(--radius-sm);
  position: relative;
  cursor: crosshair;
  overflow: hidden;
  background:
    linear-gradient(to bottom, transparent, #000),
    linear-gradient(to right, #fff, hsl(var(--cp-hue), 100%, 50%));
}

.cp-gradient__thumb {
  position: absolute;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid #fff;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.25);
  transform: translate(-50%, -50%);
  pointer-events: none;
  /* left and top set by JS from S and V values */
}

/* Sliders */
.cp-slider {
  -webkit-appearance: none;
  width: 100%;
  height: 10px;
  border-radius: var(--radius-full);
  outline: none;
  cursor: pointer;
}

.cp-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.2), 0 1px 3px rgba(0, 0, 0, 0.15);
  cursor: grab;
}

.cp-hue-slider::-webkit-slider-runnable-track {
  height: 10px;
  border-radius: var(--radius-full);
  background: linear-gradient(to right,
    hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%),
    hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), hsl(360,100%,50%));
}

/* Mini inputs */
.cp-input {
  height: 24px;
  padding: 0 6px;
  background: var(--color-surface-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono); /* monospace for color values */
  font-size: var(--text-xs);
  color: var(--color-text-primary);
  text-align: center;
  min-width: 0;
  flex: 1;
}

/* Swatch */
.cp-swatch {
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  border: 1px solid rgba(0, 0, 0, 0.1);
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
}
.cp-swatch:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 1px;
}

@media (prefers-reduced-motion: reduce) {
  .color-picker-panel { animation-duration: 0.01ms !important; }
}
```

---

## Eyedropper — implementation note

The `EyeDropper` API is available in Chromium-based browsers (Chrome 95+, Edge 95+). In a Tauri app on macOS and Windows, it is available. It is not available in Firefox or Safari.

```jsx
// Check availability at component mount
const eyedropperSupported = typeof window !== 'undefined' && 'EyeDropper' in window;

// On click:
async function pickFromScreen() {
  const eyeDropper = new EyeDropper();
  try {
    const result = await eyeDropper.open();
    // result.sRGBHex is always a #RRGGBB hex string
    applyColor(result.sRGBHex);
  } catch {
    // User cancelled — do nothing
  }
}
```

The eyedropper button is `aria-hidden` when the API is not available. It is never shown and never in the tab order in that case.

---

## Keyboard navigation within the panel

Full keyboard operability is required. Tab order within the open panel:

```
1. Gradient area       (role="slider", 2D, arrow keys)
2. Hue slider          (native range input)
3. Alpha slider        (native range input, only if alpha=true)
4. Eyedropper button   (only if API available)
5. Format switcher     (HEX / RGB / HSL buttons, arrow keys)
6. Color inputs        (text inputs, Tab between them)
7. Swatches            (only if swatches provided, arrow keys in grid)
```

Escape closes the panel and returns focus to the trigger.

---

## Accessibility requirements

- **Trigger:** `aria-haspopup="dialog"`. `aria-expanded` reflects open state. `aria-label="Color picker"` + screen-reader-visible text showing current value.
- **Panel:** `role="dialog"`. `aria-label="Color picker"`.
- **Gradient area:** `role="slider"`. `aria-label="Saturation and brightness"`. `aria-valuetext` describes both axes: `"Saturation 90%, Brightness 60%"`. Updated on every change.
- **Hue slider:** native `<input type="range">`. `aria-label="Hue"`. `aria-valuetext="220 degrees"`.
- **Alpha slider:** `aria-label="Opacity"`. `aria-valuetext="80%"`.
- **Format inputs:** `aria-label` per input: `"Hex color"`, `"Red"`, `"Green"`, `"Blue"`, `"Alpha"`, `"Hue"`, `"Saturation"`, `"Lightness"`.
- **Swatches:** Each swatch is a `<button>` with `aria-label` = the swatch label or its color value string. `aria-pressed` when the swatch matches the current color.
- **Color communication:** Color is inherently a visual concept. Screen reader users will hear the color value string (e.g. "#2563EB"). The trigger always shows the value as visible text so it can be read by AT. Do not rely on the swatch appearance alone to communicate the current value.

---

## Anti-patterns

**Using ColorPicker for status selection.** If the user is picking a "status color" from a fixed set (e.g., red / yellow / green), use RadioGroup or SegmentedControl with colored options. ColorPicker is for freeform color values.

**Firing expensive operations in `onChange`.** `onChange` fires on every drag frame — dozens of times per second. API calls, persistence, and validation belong in `onChangeEnd`.

**Displaying the panel inline (not in a portal).** ColorPicker panel must render in a portal. Any `overflow: hidden` parent — including sidebar panels, card containers, and scrollable lists — will clip the panel.

**Alpha slider without communicating transparency in the trigger.** When `alpha=true` and the color has opacity, the trigger swatch should render the color on a checkerboard background so the user can see the transparency. A solid swatch at 50% opacity looks identical to a different solid color.

**Locking the format.** If the consumer passes `format="hex"` but does not expose the format switcher UI, users cannot see or edit RGB or HSL values. This is acceptable. But the internal conversion must still use HSV as the intermediate state — not by parsing one output format and converting to another.

---

## Implementation checklist

- [ ]  Trigger renders current color as swatch + value text (swatch-input) or square (swatch-only)
- [ ]  Trigger shows chevron on hover to communicate it opens a panel
- [ ]  Panel renders in portal at Level 3 elevation
- [ ]  Panel auto-positions below trigger, flips if insufficient viewport space
- [ ]  Gradient area background updates as hue slider changes
- [ ]  Gradient thumb position derived from S and V values
- [ ]  Click anywhere in gradient area updates S + V
- [ ]  Drag on gradient area updates S + V continuously (no transition on thumb during drag)
- [ ]  Gradient area keyboard: arrow keys ±1%, shift+arrow ±10%
- [ ]  Hue slider range 0–360, updates gradient area background
- [ ]  Alpha slider shown only when `alpha=true`; range 0–100
- [ ]  Format switcher switches between HEX / RGB / HSL input sets
- [ ]  Hex input parses on blur/Enter; shows error border on invalid value
- [ ]  Numeric inputs (RGB, HSL, alpha) clamp to valid ranges
- [ ]  All inputs use font-mono
- [ ]  Eyedropper button shown only when `window.EyeDropper` exists
- [ ]  Eyedropper `aria-hidden` when not available
- [ ]  Swatches grid hidden when `swatches` is empty
- [ ]  Swatch `aria-pressed` when swatch value matches current color
- [ ]  `onChange` fires on every gradient drag frame
- [ ]  `onChangeEnd` fires only on mouseup and input blur
- [ ]  Trigger swatch shows checkerboard when alpha < 100% and `alpha=true`
- [ ]  Trigger color preview transitions at 150ms ease-out
- [ ]  Panel enter: translateY(-4px) + opacity, 150ms ease-out
- [ ]  Panel exit: opacity fade, 100ms ease-in
- [ ]  Escape closes panel, focus returns to trigger
- [ ]  Tab order within panel: gradient → hue → alpha → eyedropper → format → inputs → swatches
- [ ]  Gradient area role="slider" with aria-valuetext for both axes
- [ ]  Disabled state: opacity 0.4, panel cannot open
- [ ]  `prefers-reduced-motion` suppresses panel animation only; gradient drag is always instant