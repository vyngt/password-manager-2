# Component System Rules & Philosophy

## Why a Component Contract Exists

Token systems define *what values exist*. Component contracts define *how those values are used*. Without a contract, each component becomes an island — one developer exposes a `color` prop, another exposes a `variant` prop, a third hardcodes the color entirely.

The contract answers three questions before a single line of code is written:

1. What can the **consumer** control?
2. What does the **component** decide internally?
3. What is decided by the **token system** and cannot be overridden?

---

## The Three-Tier Ownership Model

Every visual property belongs to exactly one owner.

| Tier | Owner | Rule | Examples |
| --- | --- | --- | --- |
| **A** | Token system | Component reads a CSS variable. Consumer cannot override via props. | Surface colors, text tiers, border radius, duration tokens |
| **B** | Component | Component decides based on variant and state. Consumer cannot override. | Which surface level to use, hover vs default difference |
| **C** | Consumer | Consumer passes a prop. Component maps it to tokens. | variant, size, disabled, loading |

> **Consumers choose intent, never value.** A consumer says `variant="danger"` — never `color="#DC2626"`. The mapping from intent to token is the component's responsibility.
> 

---

## Allowed Props

### Standard Prop Vocabulary

Every component draws from the **same vocabulary**. Inventing new prop names for existing concepts is not permitted.

```
variant     Semantic intent. Always an enum, never a color value.
            Values: primary | secondary | ghost | danger | warning

size        Physical scale.
            Values: sm | md | lg

disabled    Boolean. Inert + visually muted.
            Always via CSS opacity + pointer-events, never removed from DOM.

loading     Boolean. Replaces content with spinner, implies disabled.
            Only for components triggering async actions.

class       Escape hatch for layout overrides ONLY.
            Allowed: margin, width, flex, grid properties.
            NOT allowed: color, background, font, border, padding, height.

children    Content slot.
```

### Props That Are Never Allowed

```
color       Never. Use variant instead.
style       Never as a general escape hatch.
font-size   Never. Size comes from the size prop → height tokens.
width       Never as a direct prop. Width belongs to layout.
```

---

## How Tokens Are Consumed Inside a Component

### The Mapping Rule

A component never reads a primitive token or raw CSS variable encoding a value. It reads **semantic tokens**.

```css
/* Wrong — reads a raw value */
.btn { background: var(--color-blue-600); }
.btn { background: var(--root-primary); }

/* Correct — reads a role */
.btn--primary { background: var(--color-primary); }
.btn--primary { color: var(--color-primary-foreground); }
```

### Surface Selection Rule

Every component sits at a **defined elevation level**. That level is set once in the spec, not chosen per-render.

```
Interactive inline (button, input, badge)   → surface.base or surface.1
Persistent containers (card, sidebar)        → surface.1
Hover / temporarily elevated                 → surface.2
Selected / active                            → surface.3
Floating (dropdown, tooltip, popover)        → surface.4
Modal (dialog, sheet)                        → surface.4 + scrim
```

A button inside a card is still on `surface.base` — it does not inherit the card's elevation.

### Text Role Rule

Every text element inside a component uses a **semantic text role**. The role determines size, weight, and color tier simultaneously.

```
Component titles     → role.card-title     (text-lg / medium)
Body / descriptions  → role.body           (text-sm / regular / text.primary)
Labels               → role.label          (text-sm / medium / text.secondary)
Helper / captions    → role.caption        (text-xs / regular / text.secondary)
Timestamps / meta    → role.caption        (text-xs / regular / text.tertiary)
Section overlines    → role.overline       (text-xs / medium / uppercase / text.tertiary)
Passwords / code     → role.password       (text-sm / regular / font-mono)
```

---

## Variant Rules

### The Standard Variant Set

Five variants cover every semantic case. Adding a new one requires justification — most cases that seem to need a new variant actually need a different component.

```
primary     The single most important action on the current surface.
            At most one primary element visible at a time.
            Background: --color-primary  |  Text: --color-primary-foreground

secondary   Supporting actions. Appears alongside primary.
            Background: --color-surface-2  |  Text: --color-text-primary
            Border: --color-border

ghost       Lowest emphasis. Tertiary actions, navigation items.
            No background until hover. No border.
            Text: --color-text-secondary

danger      Destructive or irreversible actions. Delete, revoke, wipe.
            Background: --color-danger  |  Text: --color-danger-foreground
            Rule: always 24px+ from other actions. Never the only button.

warning     Caution actions. Reversible but consequential.
            Background: --color-warning  |  Text: --color-warning-foreground
```

---

## Size Rules

Three sizes. Heights are derived from the typography system — not chosen independently.

| Size | Height | Font | Padding-X | Use |
| --- | --- | --- | --- | --- |
| sm | 28px | text-xs (12px) | 10px | Compact tables, dense lists |
| **md** | **36px** | **text-sm (14px)** | **16px** | **Default — used when no size specified** |
| lg | 44px | text-base (16px) | 20px | Primary CTA, unlock screen |

Default is always `md`. Specify size only when explicitly needing sm or lg.

---

## State Rules

### All Five States Are Required

Skipping a state is a bug, not a simplification.

```
default     Resting state.

hover       Cursor over element. Background shifts one surface level up.
            Transition: background-color, 150ms, ease-out.
            Never change size, position, or layout on hover.

focus       Keyboard focus. Visible focus ring: 2px solid --color-primary,
            outline-offset: 2px.
            Appears INSTANTLY (0ms). Never hidden, never animated.

active      Being pressed. transform: scale(0.97).
            Duration: 80ms, ease-out. Applies to entire component.

disabled    Inert. opacity: 0.4, cursor: not-allowed, pointer-events: none.
            Via HTML disabled attribute. Do NOT remove from DOM.
            Do NOT change color separately — opacity alone is sufficient.
```

### Loading State

```
loading     Implies disabled. Replaces content with spinner + optional label.
            Spinner: 13px (sm/md), 16px (lg). Stroke: 2px.
            Color: currentColor. Rotation: linear 700ms
            (the only legitimate use of linear easing).
```

### States That Must NOT Animate

```
disabled    Instant — animated disable feels reluctant
focus ring  Instant — delayed focus ring feels broken
error state Instant — user just submitted, needs immediate feedback
```

---

## Composition Rules

### Components Do Not Style Their Children

```
Wrong:   <Card> applies .card-title to its first <h2> child
Correct: <Card> exposes a title slot. <CardTitle> applies role.card-title internally.
```

### Components Do Not Manage Their Own Layout

```
Wrong:   <Button margin-top="24px" />    — component managing its position
Wrong:   <Button align="right" />        — component managing parent alignment

Correct: <div style="margin-top: 24px"><Button /></div>
```

### Compound Components Over Prop-Heavy Components

```jsx
// Wrong — too many props, hidden structure
<Card title="GitHub" subtitle="john@email.com" headerIcon={<Avatar>} footer={<Button>} />

// Correct — structure is visible
<Card>
  <CardHeader>
    <FaviconAvatar domain="github.com" />
    <CardTitle>GitHub</CardTitle>
    <CardSubtitle>john@email.com</CardSubtitle>
  </CardHeader>
  <CardBody>...</CardBody>
  <CardFooter>
    <Button variant="primary">Edit</Button>
  </CardFooter>
</Card>
```

---

## The Escape Hatch Rule

The **only** permitted escape hatch is the `class` prop for layout properties.

```
Permitted via class:
  margin, padding (external only), width / max-width / min-width
  flex-grow, flex-shrink, align-self, grid-column, grid-row

Not permitted via class:
  color, background, border-color    → use variant
  font-size, font-weight             → use size, or a text role
  padding-x, padding-y              → if wrong, the size prop is wrong
  border-radius                      → comes from the radius token, not configurable
```

If correct behavior requires overriding a visual property the escape hatch doesn't cover — **add a new variant or extend the API.** Do not bypass the system.

---

## Documentation Requirement

Every component must have a spec written **before implementation**:

```
Name           Consistent name, everywhere.
Purpose        One sentence: what problem does it solve.
Variants       Which variants and what each communicates.
Sizes          Which sizes and when to use each.
States         default / hover / focus / active / disabled / loading
Props          Full list with types, defaults, constraints.
Token mapping  Which tokens each variant/state uses.
Composability  Can it be nested? What can it contain?
Accessibility  Keyboard behavior, ARIA attributes, focus management.
Do not use     Explicit anti-patterns to avoid.
```

If a property cannot be specified, the design is incomplete and implementation should not begin.

---

## 10 Rules — No Exceptions

1. Consumers choose intent (variant, size, state). Components choose tokens. Token system chooses values.
2. No component reads a primitive token or a raw hex value.
3. Every text element uses a semantic role. Font size and weight are never set directly.
4. Every component sits at a defined elevation level. It does not inherit from ancestors.
5. All five interactive states are required. None are optional.
6. Focus rings appear instantly. Never hidden, reduced, or animated.
7. Components do not manage their own position in a parent layout.
8. Compound components over prop-heavy components when structure has multiple distinct regions.
9. The only escape hatch is the `class` prop for layout properties. Visual overrides are not permitted.
10. Specification is written before implementation. Unspecified behavior is unimplemented behavior.