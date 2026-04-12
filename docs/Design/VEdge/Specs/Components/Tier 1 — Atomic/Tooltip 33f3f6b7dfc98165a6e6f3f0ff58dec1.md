# Tooltip

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

A short contextual label that appears near a trigger element to name or clarify it. Tooltip answers one question: *what is this?* It is the only component in the system whose sole job is to provide information — it never performs an action, navigates, or contains interactive content.

The two canonical use cases are icon-only buttons (where there is no visible label to read) and truncated text (where the full string is clipped). In both cases, Tooltip makes hidden content accessible without consuming permanent layout space.

> **Important:** For icon-only buttons in production, prefer `TooltipIconButton` (Tier 2) over manually composing Tooltip + IconButton. The compound handles `aria-describedby` wiring, focus management, and keyboard behavior in one unit.
> 

---

## The three ownership boundaries

**The token system decides:** surface color, shadow, border, border radius, typography, animation duration, and easing curves. Tooltip reads Level 3 floating elevation tokens — it never hardcodes a hex value or pixel size for anything the token system defines.

**The Tooltip decides:** show/hide timing (hover delay vs. focus delay), animation direction (derived from placement), positioning logic (preferred side, flip behavior, viewport collision), and how it injects `aria-describedby` onto its trigger. These are internal rules — not exposed as props.

**The consumer decides:** content text, trigger element (via children slot), preferred placement side, and whether to disable the show delay for cases where immediate feedback matters. The consumer never controls color, size, or animation.

---

## Anatomy

```
[tooltip-root — wrapper element, manages positioning context]
  [trigger slot]       ← the wrapped element — button, icon, text
  [tooltip-panel]      ← the floating label (hidden until shown)
    [tooltip-content]  ← text only, max-width: 200px
    [tooltip-arrow?]   ← optional 6px caret pointing at trigger
```

The tooltip panel is rendered outside the normal DOM flow — appended to `document.body` or a portal target. This prevents clipping by `overflow: hidden` ancestors (sidebar panels, scrollable lists, card containers).

**Arrow:** The directional caret is optional. Use it when the tooltip appears away from the trigger's natural reading axis (left/right placement) or when the trigger is small and the tooltip could be visually ambiguous about what it refers to. Do not use it for tooltips that appear directly above or below large buttons — proximity is sufficient.

---

## Placement

Four preferred sides. The Tooltip resolves which to use at runtime — the consumer's `placement` prop is a preference, not a guarantee.

| Placement | Tooltip position | Arrow direction |
| --- | --- | --- |
| `top` (default) | Above trigger, center-aligned | Points down |
| `bottom` | Below trigger, center-aligned | Points up |
| `left` | Left of trigger, vertically centered | Points right |
| `right` | Right of trigger, vertically centered | Points left |

**Offset from trigger:** 8px (`--space-2`). Fixed — not configurable.

**Flip behavior:** If the preferred side has insufficient viewport clearance (< 8px between tooltip edge and viewport edge), the Tooltip flips to the opposite side automatically. If both sides are constrained, it falls back to whichever has more space.

**Alignment:** Center-aligned with the trigger by default. If center-alignment would push the tooltip past the viewport edge, shift it inward — but keep the arrow pointing at the trigger's midpoint.

**Viewport edge clearance:** Minimum 8px between tooltip panel and viewport edge on all sides.

---

## Timing

Tooltip timing is the most consequential design decision in this spec. Get it wrong and the tooltip either fires constantly (annoying) or feels broken (too slow).

### Show delay

| Trigger method | Show delay | Reason |
| --- | --- | --- |
| Hover | **500ms** | Prevents tooltip spam when scanning across a toolbar |
| Keyboard focus | **0ms** | Keyboard users need immediate feedback — no cursor to move |

The 500ms hover delay is non-negotiable. 300ms is too fast — tooltips fire during casual cursor movement. 700ms feels broken. 500ms is the value used by every major design system (Radix, Floating UI, Material, Fluent).

The `delay` prop allows overriding the hover delay for individual tooltips. Use sparingly — the only justified case is a first-run experience where the tooltip teaches a UI element and must be noticed.

### Hide delay

**100ms grace period** after cursor leaves the trigger before the tooltip closes. This prevents tooltip flicker when the cursor briefly crosses the trigger edge during normal movement. After the grace period, the tooltip exits with a 100ms `ease-in` fade.

On blur (keyboard), hide delay is 0ms — the tooltip disappears when focus moves away.

**Group behavior (warm period):** If the cursor moves from one tooltipped element to another without leaving the tooltip zone, the second tooltip appears immediately — the 500ms delay does not restart. This is the "warm period" behavior. Once the cursor leaves all tooltipped elements and the grace period expires, the delay resets to 500ms.

---

## Motion

Enter and exit are asymmetric — in line with the motion framework's entrance/exit easing rule.

### Enter

```
transform: translateY(-4px) + opacity(0)  →  translateY(0) + opacity(1)
duration:  var(--duration-fast)  150ms
easing:    var(--ease-out)       cubic-bezier(0.16, 1, 0.3, 1)
```

The 4px offset is direction-aware:

```
placement=top:    translateY(+4px) → translateY(0)   tooltip enters from below
placement=bottom: translateY(-4px) → translateY(0)   tooltip enters from above
placement=left:   translateX(+4px) → translateX(0)   tooltip enters from right
placement=right:  translateX(-4px) → translateX(0)   tooltip enters from left
```

The tooltip always appears to come from the direction of its trigger — not from an arbitrary direction. This reinforces the spatial relationship between tooltip and trigger.

### Exit

```
opacity(1) → opacity(0)
duration:  100ms
easing:    var(--ease-in)   cubic-bezier(0.55, 0, 1, 0.45)
```

Exit is opacity-only — no transform. The tooltip fades out without moving. This is intentional: a tooltip that slides away draws unnecessary attention to its departure. The user has already moved on.

Exit is also faster than enter (100ms vs 150ms) because the tooltip is no longer relevant once the cursor has moved.

---

## Elevation — Level 3 Floating

Tooltip sits at **Level 3 — floating** in the elevation system. This is the same level as dropdowns, popovers, and context menus.

```css
.tooltip-panel {
  background:   var(--color-surface-4);
  border:       0.5px solid var(--color-border-strong);
  box-shadow:   var(--shadow-md);
  border-radius: var(--radius);   /* 6px */
}
```

In light mode, `--shadow-md` is `0 4px 12px rgba(0,0,0,0.12)`. In dark mode, it resolves to a 1px light ring (`0 0 0 1px rgba(255,255,255,0.10)`). The component reads the token and never knows which it got.

---

## Content rules

**Text only.** No icons, images, buttons, links, or any interactive element inside a tooltip. A tooltip that contains a link is not a tooltip — it is a Popover (Tier 2).

**One sentence maximum.** Tooltip content should be a noun phrase or short imperative: "Copy to clipboard", "Lock vault", "Collapse sidebar". If the content requires a full sentence with a period, it is too long for a tooltip.

**No title-casing.** Sentence case only: "Copy password", not "Copy Password".

**Max width: 200px.** Content that wraps to three or more lines at 200px is too long for a tooltip. Use a Popover with richer formatting instead.

**No HTML inside content.** The consumer passes a plain string — never a JSX or HTML tree. Tooltip sanitizes its content and renders it as text.

---

## Typography

Tooltip uses `role.caption` — the smallest text role in the system.

```
font-size:   var(--text-xs)              12px
line-height: var(--text-xs--line-height) 16px
font-weight: var(--font-weight-normal)   400
color:       var(--color-text-primary)
```

Padding: `6px 10px` (`--space-1_5` vertical, between `--space-2` and `--space-3` horizontal). This is intentionally tighter than other components — tooltip is informational, not interactive, and should feel compact.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `content` | `string` | — | **Required.** The tooltip label text. Plain string only — no JSX. |
| `placement` | `"top" / "bottom" / "left" / "right"` | `"top"` | Preferred side. Auto-flips if insufficient viewport space. |
| `delay` | `number` | `500` | Hover show delay in ms. Does not affect keyboard focus delay (always 0ms). |
| `disabled` | `boolean` | `false` | Suppresses tooltip entirely. Use when the trigger has a visible label that makes the tooltip redundant. |
| `arrow` | `boolean` | `false` | Renders a directional caret pointing at the trigger. |
| `children` | `Slot` | — | **Required.** The trigger element. Must be a single focusable element. |

---

## Token mapping

### Elevation tokens consumed

```
--color-surface-4       tooltip background
--color-border-strong   0.5px border
--shadow-md             drop shadow (light) or ring (dark)
--radius                6px border-radius
```

### Typography tokens consumed

```
--font-sans
--text-xs / --text-xs--line-height   12px / 16px
--font-weight-normal                 400
--color-text-primary                 tooltip text
```

### Motion tokens consumed

```
--duration-fast   150ms   enter transition
--ease-out        cubic-bezier(0.16, 1, 0.3, 1)   enter easing
--ease-in         cubic-bezier(0.55, 0, 1, 0.45)  exit easing
```

Exit duration (100ms) has no dedicated token — it falls between `--duration-micro` (80ms) and `--duration-fast` (150ms) and is written inline as `100ms`.

---

## CSS implementation reference

```css
/* tooltip.css */

.tooltip-panel {
  position: absolute;    /* positioned by JS, via portal */
  z-index: 50;
  max-width: 200px;
  padding: 6px 10px;
  background: var(--color-surface-4);
  border: 0.5px solid var(--color-border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-md);
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  line-height: var(--text-xs--line-height);
  font-weight: var(--font-weight-normal);
  color: var(--color-text-primary);
  pointer-events: none;   /* tooltip is never interactive */
  white-space: normal;
  word-break: break-word;
}

/* Entry animations — direction-aware, set by JS placement result */
.tooltip-panel[data-placement="top"] {
  transform-origin: bottom center;
}
.tooltip-panel[data-placement="bottom"] {
  transform-origin: top center;
}

@keyframes tooltip-enter-top {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}
@keyframes tooltip-enter-bottom {
  from { opacity: 0; transform: translateY(-4px); }
  to   { opacity: 1; transform: translateY(0); }
}
@keyframes tooltip-enter-left {
  from { opacity: 0; transform: translateX(4px); }
  to   { opacity: 1; transform: translateX(0); }
}
@keyframes tooltip-enter-right {
  from { opacity: 0; transform: translateX(-4px); }
  to   { opacity: 1; transform: translateX(0); }
}
@keyframes tooltip-exit {
  from { opacity: 1; }
  to   { opacity: 0; }
}

.tooltip-panel[data-state="open"][data-placement="top"] {
  animation: tooltip-enter-top var(--duration-fast) var(--ease-out) forwards;
}
.tooltip-panel[data-state="open"][data-placement="bottom"] {
  animation: tooltip-enter-bottom var(--duration-fast) var(--ease-out) forwards;
}
.tooltip-panel[data-state="open"][data-placement="left"] {
  animation: tooltip-enter-left var(--duration-fast) var(--ease-out) forwards;
}
.tooltip-panel[data-state="open"][data-placement="right"] {
  animation: tooltip-enter-right var(--duration-fast) var(--ease-out) forwards;
}
.tooltip-panel[data-state="closed"] {
  animation: tooltip-exit 100ms var(--ease-in) forwards;
}

/* Arrow */
.tooltip-arrow {
  position: absolute;
  width: 6px;
  height: 6px;
  background: var(--color-surface-4);
  border: 0.5px solid var(--color-border-strong);
  transform: rotate(45deg);
  /* JS positions the arrow based on placement and trigger midpoint */
}

/* Arrow border masking — hide the interior-facing border edges */
[data-placement="top"]    .tooltip-arrow { border-top: none; border-left: none; }
[data-placement="bottom"] .tooltip-arrow { border-bottom: none; border-right: none; }
[data-placement="left"]   .tooltip-arrow { border-bottom: none; border-left: none; }
[data-placement="right"]  .tooltip-arrow { border-top: none; border-right: none; }

@media (prefers-reduced-motion: reduce) {
  .tooltip-panel { animation-duration: 0.01ms !important; }
}
```

---

## Composability

### Tooltip wraps its trigger

Tooltip is a wrapper component — it takes the trigger as its `children` slot and handles all wiring internally:

```jsx
// Correct
<Tooltip content="Lock vault now">
  <IconButton icon={Lock} aria-label="Lock" />
</Tooltip>

// Wrong — tooltip not wired to trigger
<IconButton icon={Lock} aria-label="Lock" />
<Tooltip content="Lock vault now" />
```

The trigger must be a single focusable element. Tooltip injects `aria-describedby` pointing to the tooltip panel's `id`. It does not inject `aria-label` — that would replace the element's accessible name, not supplement it.

### Tooltip is NOT a label replacement

A visible label is always better than a tooltip. Tooltip is for:

- Icon-only buttons where space is constrained
- Truncated text that is cut off by layout
- Supplementary context for an already-labelled element

It is not for:

- Naming buttons that have room for a text label
- Delivering critical instructions (tooltip is not always discovered)
- Explaining an entire feature

### Tooltip + IconButton → TooltipIconButton (Tier 2)

For the common case of an icon-only button with a tooltip, use `TooltipIconButton` from Tier 2. It composes Tooltip and IconButton correctly, ensures the `aria-label` and tooltip `content` stay in sync, and handles the edge case where the button is disabled (tooltip still shows on hover — the user still needs to know what the button does).

```jsx
// Preferred for icon buttons
<TooltipIconButton icon={Copy} label="Copy password" />

// Manual composition — only when TooltipIconButton does not fit the context
<Tooltip content="Copy password">
  <IconButton icon={Copy} aria-label="Copy password" />
</Tooltip>
```

### Tooltip on disabled elements

By default, disabled elements do not fire mouse events — so hover tooltips do not trigger. To show a tooltip on a disabled button (e.g., explaining *why* it is disabled), wrap the trigger in a `<span>` that does receive mouse events:

```jsx
<Tooltip content="Requires at least one entry selected">
  <span style="display: inline-flex;">
    <Button variant="primary" disabled>Export</Button>
  </span>
</Tooltip>
```

This is a known exception. The span is `aria-hidden` and the tooltip is still discoverable via keyboard focus on the button (which does receive focus even when disabled via `tabindex="0"` override — see Accessibility section).

---

## Accessibility requirements

- **`role="tooltip"` on the panel.** Screen readers announce the tooltip content when the trigger is focused.
- **`aria-describedby` on the trigger.** Points to the tooltip panel's `id`. This supplements — does not replace — the trigger's `aria-label` or visible text.
- **Keyboard accessible.** Tooltip appears on `:focus-visible` with 0ms delay. Disappears on blur. No keyboard interaction required to dismiss — Tab away is sufficient.
- **Not a substitute for `aria-label`.** Do not remove `aria-label` from an IconButton and rely on Tooltip alone. Screen readers that do not surface `role="tooltip"` content would lose the accessible name entirely.
- **Disabled buttons still show tooltip on focus.** If a button is disabled but should still explain its purpose, add `tabindex="0"` to keep it in the focus order and let Tooltip show on focus. This is the correct pattern for "explain why disabled" scenarios.
- **Touch devices.** Tooltip is hover/focus only — it has no touch trigger. Any information conveyed only via tooltip must also be available another way on touch. In Tauri (desktop-only), this is not a concern; note it anyway for correctness.
- **No tooltip on tooltip.** Tooltip panels are `pointer-events: none` — they cannot themselves be hovered or focused.

---

## Anti-patterns

**Interactive content inside Tooltip.** Buttons, links, and inputs inside a tooltip break the pointer-events model and the accessibility contract. Use a Popover (Tier 2) instead.

**Using Tooltip as a label for a labelled element.** If a button already has a visible text label, adding a tooltip that says the same thing is noise. If it says something different — that is the label, and the button text is wrong.

**Relying on Tooltip for critical information.** Tooltips are not always discovered. Critical instructions belong in helper text, inline labels, or modals — not tooltips.

**Setting `delay={0}` globally.** The 500ms delay exists to prevent tooltip spam. Removing it causes every cursor movement across the interface to trigger tooltip popups. Override `delay` only for specific, justified cases.

**Tooltip on non-focusable elements.** Tooltip only works on elements that can receive keyboard focus. A `<div>` with an `onClick` is not sufficient. The trigger must be an interactive element — `<button>`, `<a>`, or an element with `tabindex="0"`.

**Long text.** If tooltip content wraps past two lines at 200px max-width, it is not tooltip content. Shorten the label or switch to a Popover.

**Nesting tooltips.** A tooltip inside a tooltip is undefined behavior. The outer tooltip's `pointer-events: none` panel would make the inner trigger unreachable anyway.

---

## Implementation checklist

- [ ]  Tooltip panel renders in a portal (outside DOM flow, appended to body)
- [ ]  Hover show delay: 500ms; keyboard focus show delay: 0ms
- [ ]  Hide grace period: 100ms after cursor leaves trigger
- [ ]  Group warm period: moving between tooltipped elements skips the 500ms delay
- [ ]  Enter animation is direction-aware (4px offset toward trigger)
- [ ]  Exit animation is opacity-only fade, 100ms ease-in
- [ ]  Placement auto-flip when preferred side has < 8px viewport clearance
- [ ]  Offset from trigger: 8px fixed
- [ ]  Alignment: center-aligned with trigger, shifted inward at viewport edges
- [ ]  Content is plain text only — no JSX accepted
- [ ]  Max width: 200px with wrapping
- [ ]  `pointer-events: none` on tooltip panel
- [ ]  `role="tooltip"` on panel
- [ ]  `aria-describedby` injected onto trigger pointing to panel ID
- [ ]  Tooltip shows on `:focus-visible` (keyboard) with 0ms delay
- [ ]  Tooltip hides on blur with 0ms delay
- [ ]  `disabled` prop suppresses tooltip entirely
- [ ]  Elevation: `--color-surface-4`, `--shadow-md`, `0.5px --color-border-strong`
- [ ]  Arrow renders and positions correctly for all four placements
- [ ]  Arrow border masking hides interior-facing edges
- [ ]  `prefers-reduced-motion` suppresses animation (tooltip appears/disappears instantly)
- [ ]  Works correctly inside `overflow: hidden` containers (portal prevents clipping)
- [ ]  Tooltip on disabled button: wrapper span pattern documented; focus still shows tooltip