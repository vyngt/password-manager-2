# Tier 1 — Atomic

## Overview

Tier 1 components are the smallest indivisible units in the system. Each component:

- Reads CSS custom properties from the token system directly
- Manages its own visual states (default, hover, focus, active, disabled, loading)
- Has no children that are themselves components
- Is completely agnostic about its parent layout

**No Tier 1 component imports another component.** If implementation requires composing two Tier 1 components, the result belongs in Tier 2.

---

## Interactive

| Component | Purpose | Variants | Status |
| --- | --- | --- | --- |
| Button | Primary action primitive. Every clickable action that does not navigate is a Button. | primary · secondary · ghost · danger · warning | ✅ Specced |
| Icon Button | Icon-only square button. No label. `aria-label` required. `title` fallback tooltip. Production use requires `TooltipIconButton` (Tier 2). | primary · secondary · ghost · danger · warning | ✅ Specced |
| Toggle / Switch | Binary state control for settings and preferences. | on · off | Pending |
| Checkbox | Multi-select control. Supports indeterminate state for parent-of-group. | checked · indeterminate · unchecked | Pending |
| Select | Single-choice dropdown. Trigger + floating list. | — | Pending |

---

## Form

| Component | Purpose | Variants | Status |
| --- | --- | --- | --- |
| Input | Single-line text entry. Wraps native `<input>` with design tokens. | text · password · search | Pending |
| Label | Field label. Always paired with an Input. Never used as standalone text. | — | Pending |
| Helper Text | Supplementary field feedback positioned below an Input. | hint · error · success | Pending |
| Password Strength Meter | Four-segment visual indicator of password entropy. | — | Pending |
| Textarea | Multi-line text entry with auto-resize and max-height constraint. | — | Pending |

---

## Display

| Component | Purpose | Variants | Status |
| --- | --- | --- | --- |
| Badge | Small label for status, count, or category. Inline or standalone. | default · info · success · warning · danger | Pending |
| Avatar | Visual identity for a user (initials), domain (favicon), or image. | favicon · initials · image | Pending |
| Separator | Visual divider. Horizontal for sections, vertical for inline groups. | horizontal · vertical · labeled | Pending |
| Spinner | Loading indicator. 700ms linear rotation — the only permitted linear animation. | sm · md · lg | Pending |
| Kbd | Keyboard shortcut hint. Monospace, styled like a physical key. | — | Pending |

---

## Overlay / Feedback

| Component | Purpose | Elevation | Status |
| --- | --- | --- | --- |
| Tooltip | Short contextual label for icon-only buttons and truncated text. Appears on hover/focus. | Level 3 | Pending |
| Toast | Transient notification. Enters 300ms ease-out, exits 150ms ease-in. Auto-dismisses. | Level 3 | Pending |
| Focus Ring | System-level keyboard focus indicator. 2px solid, 0ms (instant), never hidden. | — | Pending |

---

## Component specs

[Button](Tier%201%20%E2%80%94%20Atomic/Button%2033f3f6b7dfc98108b6ecc7f7b99c0a58.md)

[IconButton](Tier%201%20%E2%80%94%20Atomic/IconButton%2033f3f6b7dfc9812bb2f0d923393acef1.md)