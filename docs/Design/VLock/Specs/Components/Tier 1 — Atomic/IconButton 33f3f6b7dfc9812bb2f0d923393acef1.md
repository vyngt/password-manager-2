# IconButton

**Status:** Canonical · **Version:** 1.0 · **Layer:** Tier 1 — Atomic

---

## Purpose

An icon-only button. Used for actions where the icon communicates intent clearly without a label — copy to clipboard, close panel, open settings, delete row, show more options.

IconButton is **not** a convenience shorthand for `Button` with an icon. It is a separate component with different sizing logic, no label slot, and stricter accessibility requirements. The two components serve different roles and must not substitute for each other.

**Use IconButton when:**

- The icon is universally recognized in context (copy, close, settings, kebab, search, edit)
- Space is constrained and a label would add visual noise without meaning (table row actions, toolbar, list item hover actions)
- Multiple similar actions appear in a dense group and labels would cause crowding

**Use Button with `leadingIcon` instead when:**

- The icon alone does not self-evidently communicate the action
- The action is destructive — danger must be communicated through label text, not only icon + red color
- First-encounter or onboarding contexts where users may not recognize the icon
- The action is the primary CTA on a surface

---

## The three ownership boundaries

**The token system decides:** surface colors, text color tiers, border radius, duration values, easing curves, focus ring. The component reads CSS custom properties and never hardcodes these values.

**The component decides:** square dimensions (width always equals height), icon centering via flexbox, spinner sizing, `title` attribute population from `aria-label` as native tooltip fallback.

**The consumer decides:** variant, size, icon content, `aria-label` (required), tooltip text override, disabled state, loading state.

**Additional constraint not in Button:** `aria-label` is never optional. A rendered IconButton without `aria-label` is an accessibility violation. Implementations should throw a runtime warning if the prop is absent in development.

---

## Variants

Five variants. Identical semantic mapping to Button. **Default is `ghost`** — not `secondary`.

The reasoning: the large majority of icon-only actions in a desktop UI are low-emphasis tertiary actions (copy, close, settings). Defaulting to `ghost` renders these without background or border at rest — the correct visual weight. A `secondary` default would mean every toolbar icon gets a visible border, creating unnecessary visual noise.

| Variant | Semantic intent | When to use |
| --- | --- | --- |
| `primary` | Single most important action on surface | Add new entry (FAB-like), prominent create |
| `secondary` | Supported, always-visible action | Toolbar action that must remain visible at rest |
| `ghost` | **Default.** Low-emphasis, tertiary | Copy, close, settings, kebab, drag handle |
| `danger` | Destructive or irreversible | Delete row, revoke token — must be visually distinct at rest |
| `warning` | Consequential but reversible | Reset field, clear value |

> **The `danger` constraint applies here too.** A danger IconButton must be separated from adjacent actions by at least 24px. Because there is no label, the red color alone carries the destructive signal — which is why `danger` IconButtons should be used only where the icon is unambiguous (trash icon, X icon on a destructive list item).
> 

---

## Sizes

Three sizes. Width always equals height — the component is always a square. Uses the same height tokens as Button so icon buttons and text buttons align in toolbars and flex rows without manual adjustment.

| Size | Dimensions | Icon size | Use |
| --- | --- | --- | --- |
| `sm` | 28×28px | 14px | Dense table row actions, compact toolbars, list item hover controls |
| `md` | 36×36px | 16px | **Default.** Standard toolbar, card header actions, sidebar controls |
| `lg` | 44×44px | 20px | Prominent actions, larger touch targets, primary zone |

Icon is always centered via `display: flex; align-items: center; justify-content: center`. No explicit padding is specified — the icon size within the container determines visual density.

**Why the same height tokens as Button:** Using identical dimensions means a row of mixed Button and IconButton components at `md` aligns on a shared 36px baseline without layout correction. If IconButton used a separate compact scale (e.g., 32px), toolbar alignment would require wrapper adjustments on every use.

---

## States

All six states are required. Rules are identical to Button except where noted.

**Default** — Resting state. Variant token values at base level.

**Hover** — Background shifts one surface step. Transition: `background-color 150ms ease-out`. `ghost` variant gains `surface-2` background and shifts text to `text-primary`. Never change size or position on hover.

**Focus** — `outline: 2px solid var(--color-focus-ring)`, `outline-offset: 2px`. Appears instantly at 0ms. Never hidden. Because the component is square, the focus ring forms a clean rectangle around it.

**Active** — `transform: scale(0.97)`, 80ms `ease-out`. Reverts on release at the same 80ms.

**Disabled** — `opacity: 0.4`, `cursor: not-allowed`, `pointer-events: none`. Via HTML `disabled` attribute. Appears instantly.

**Loading** — Icon replaced by spinner. `opacity: 0.7`. `pointer-events: none`. Add `aria-busy="true"` to root. Because there is no label, the spinner is the sole visual content — spinner size must be large enough to communicate activity clearly (see spinner sizes in CSS reference below).

---

## Tooltip requirement

Every IconButton must have an associated tooltip. Without a visible label, the tooltip is the only way a sighted user can discover the action's meaning on first encounter.

**Implementation at Tier 1 (this component):**

- `aria-label` is required and set on the `<button>` element
- `title` attribute is set to the `tooltip` prop value, falling back to `aria-label` if `tooltip` is not provided
- `title` provides a native browser tooltip as a minimal fallback

**Production requirement (Tier 2 composition):**

- Native `title` tooltips are not styled, have no animation, and behave inconsistently across browsers
- In production, every IconButton must be wrapped in the Tooltip component
- The `TooltipIconButton` compound component at Tier 2 handles this automatically
- Do not rely on `title` alone in the shipped application

```jsx
// Tier 1 — IconButton standalone (acceptable only in development / prototype)
<IconButton icon={<Copy />} aria-label="Copy password" />

// Tier 2 — Production pattern via TooltipIconButton
<TooltipIconButton icon={<Copy />} aria-label="Copy password" tooltip="Copy" />
// TooltipIconButton wraps: <Tooltip content={tooltip}><IconButton .../></Tooltip>
```

The `tooltip` prop text should be short — one to three words. `aria-label` can be longer and more descriptive ("Copy password to clipboard"). If `tooltip` is omitted, `aria-label` is used for both — keep `aria-label` short enough to work as a tooltip label in that case.

---

## Props API

| Prop | Type | Default | Description |
| --- | --- | --- | --- |
| `icon` | `Slot` | — | **Required.** The icon to render. Expected: SVG with `stroke="currentColor"` and `stroke-width="2"`. Size is set by the component via CSS — do not set width/height on the passed icon. |
| `variant` | `"primary" / "secondary" / "ghost" / "danger" / "warning"` | `"ghost"` | Semantic intent. Defaults to `ghost` — unlike Button which defaults to `secondary`. |
| `size` | `"sm" / "md" / "lg"` | `"md"` | Physical scale. Dimension is square: width = height = the corresponding height token. |
| `aria-label` | `string` | — | **Required.** Describes the action. Announced by screen readers. Used as `title` fallback if `tooltip` is not provided. |
| `tooltip` | `string` | — | Override text for the tooltip display. Defaults to `aria-label` if omitted. Should be 1–3 words. |
| `disabled` | `boolean` | `false` | Via HTML `disabled` attribute. Removes from tab order. |
| `loading` | `boolean` | `false` | Implies disabled. Replaces icon with spinner. |
| `type` | `"button" / "submit" / "reset"` | `"button"` | HTML button type. Always explicit — HTML default is `"submit"`. |
| `class` | `string` | — | Layout overrides only: margin, position in flex/grid. Visual overrides not permitted. |
| `onClick` | `EventHandler<MouseEvent>` | — | Suppressed automatically when `disabled` or `loading`. |

---

## Token mapping

### By variant

Token mapping is identical to Button. IconButton uses the same semantic tokens — the only difference is square geometry and absence of a label slot.

| Variant | State | Background | Icon color | Border |
| --- | --- | --- | --- | --- |
| `primary` | default | `--color-primary` | `--color-primary-foreground` | transparent |
| `primary` | hover | `--color-primary-hover` | `--color-primary-foreground` | transparent |
| `secondary` | default | `--color-surface-1` | `--color-text-primary` | `--color-border` |
| `secondary` | hover | `--color-surface-2` | `--color-text-primary` | `--color-border` |
| `ghost` | default | transparent | `--color-text-secondary` | transparent |
| `ghost` | hover | `--color-surface-2` | `--color-text-primary` | transparent |
| `danger` | default | `--color-danger` | `--color-danger-foreground` | transparent |
| `danger` | hover | `--color-danger-hover` | `--color-danger-foreground` | transparent |
| `warning` | default | `--color-warning` | `--color-warning-foreground` | transparent |
| `warning` | hover | `--color-warning-hover` | `--color-warning-foreground` | transparent |

### By state (all variants)

| State | Properties | Duration | Easing |
| --- | --- | --- | --- |
| focus | `outline: 2px solid --color-focus-ring`, offset 2px | **0ms** | — |
| active | `transform: scale(0.97)` | 80ms | ease-out |
| disabled | `opacity: 0.4` on root | 0ms | — |
| loading | `opacity: 0.7` on root | 0ms | — |
| hover bg | `background-color` | 150ms | ease-out |

### Tokens consumed

```jsx
// Motion
--duration-micro   80ms    active press/release
--duration-fast    150ms   hover transition
--ease-out         cubic-bezier(0.16, 1, 0.3, 1)

// Shape
--radius           6px     border-radius

// Sizing
--height-sm        28px    width and height at sm
--height-md        36px    width and height at md
--height-lg        44px    width and height at lg

// Focus
--color-focus-ring          outline color
```

---

## CSS implementation reference

```css
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: var(--radius); /* 6px */
  border: 0.5px solid transparent;
  cursor: pointer;
  user-select: none;
  outline: none;
  transition:
    background-color var(--duration-fast)  var(--ease-out),
    border-color     var(--duration-fast)  var(--ease-out),
    color            var(--duration-fast)  var(--ease-out),
    transform        var(--duration-micro) var(--ease-out);
}

/* Square sizing — width = height = height token */
.icon-btn--sm { width: var(--height-sm); height: var(--height-sm); } /* 28×28 */
.icon-btn--md { width: var(--height-md); height: var(--height-md); } /* 36×36 */
.icon-btn--lg { width: var(--height-lg); height: var(--height-lg); } /* 44×44 */

/* Icon sizing — set on direct SVG child */
.icon-btn--sm > svg { width: 14px; height: 14px; }
.icon-btn--md > svg { width: 16px; height: 16px; }
.icon-btn--lg > svg { width: 20px; height: 20px; }

/* Variants */
.icon-btn--primary   { background: var(--color-primary);   color: var(--color-primary-foreground); }
.icon-btn--secondary { background: var(--color-surface-1); color: var(--color-text-primary); border-color: var(--color-border); }
.icon-btn--ghost     { background: transparent;            color: var(--color-text-secondary); }
.icon-btn--danger    { background: var(--color-danger);     color: var(--color-danger-foreground); }
.icon-btn--warning   { background: var(--color-warning);   color: var(--color-warning-foreground); }

/* Hover */
.icon-btn--primary:hover:not(:disabled)   { background: var(--color-primary-hover); }
.icon-btn--secondary:hover:not(:disabled) { background: var(--color-surface-2); }
.icon-btn--ghost:hover:not(:disabled)     { background: var(--color-surface-2); color: var(--color-text-primary); }
.icon-btn--danger:hover:not(:disabled)    { background: var(--color-danger-hover); }
.icon-btn--warning:hover:not(:disabled)   { background: var(--color-warning-hover); }

/* Focus */
.icon-btn:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}

/* Active */
.icon-btn:active:not(:disabled):not(.icon-btn--loading) {
  transform: scale(0.97);
}

/* Disabled */
.icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

/* Loading */
.icon-btn--loading {
  opacity: 0.7;
  pointer-events: none;
}

/* Spinner — fills the icon slot */
@keyframes icon-btn-spin { to { transform: rotate(360deg); } }

.icon-btn__spinner {
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: icon-btn-spin 700ms linear infinite;
  flex-shrink: 0;
}

.icon-btn--sm .icon-btn__spinner { width: 13px; height: 13px; }
.icon-btn--md .icon-btn__spinner { width: 14px; height: 14px; }
.icon-btn--lg .icon-btn__spinner { width: 16px; height: 16px; }

@media (prefers-reduced-motion: reduce) {
  .icon-btn { transition-duration: 0.01ms !important; }
  .icon-btn__spinner { animation-duration: 0.01ms !important; }
}
```

---

## Composability

### Tier 1 constraint

IconButton does not internally compose the Tooltip component. Tooltip is a separate Tier 1 component, and Tier 1 components do not import each other.

IconButton provides `title` attribute as a native fallback. For production use, the `TooltipIconButton` compound at Tier 2 wraps this component in Tooltip and is what should appear in application code.

### Position in layouts

IconButton manages only its own internal layout. It does not manage its position within a parent. The consumer is always responsible.

```jsx
// Correct — layout owner handles spacing and alignment
<div class="flex items-center gap-1">
  <TooltipIconButton icon={<Copy />} aria-label="Copy" />
  <TooltipIconButton icon={<Eye />} aria-label="Reveal" />
  <TooltipIconButton icon={<Trash />} aria-label="Delete" variant="danger" />
</div>

// Wrong — IconButton managing its own margin
<IconButton icon={<Trash />} aria-label="Delete" class="ml-auto" variant="danger" />
```

Note that `class="ml-auto"` is an acceptable layout override via the `class` prop — see Props API. The example above is marked wrong because the comment implies the component is managing its own position relative to siblings, which should be a flex/grid layout concern.

### Grouping danger variants

When danger and non-danger IconButtons appear in the same group, the danger button must be separated by at least 24px. In a flex group, use a spacer or `ml-6` on the danger button.

```jsx
// Correct — 24px separation between non-danger and danger
<div class="flex items-center gap-1">
  <TooltipIconButton icon={<Copy />}  aria-label="Copy" />
  <TooltipIconButton icon={<Eye />}   aria-label="Reveal" />
  <div class="w-6" />  {/* 24px spacer */}
  <TooltipIconButton icon={<Trash />} aria-label="Delete" variant="danger" />
</div>
```

---

## Relationship to Button

IconButton and Button share the same variant vocabulary and identical token mappings. They are otherwise separate components.

| Property | Button | IconButton |
| --- | --- | --- |
| Label slot | Required | None |
| Icon slots | `leadingIcon`, `trailingIcon` (optional) | `icon` (required) |
| Shape | Rectangular | Always square |
| Width | Content-driven | Fixed (= height) |
| Default variant | `secondary` | `ghost` |
| `fullWidth` prop | Yes | No |
| `aria-label` | Optional | **Required** |
| Tooltip | Not required | **Required** (native fallback via `title`) |
| Tier 2 compound | — | `TooltipIconButton` |

Do not attempt to replicate IconButton behavior using `Button` with an icon and no label text. The two components have different internal layout, different sizing logic, and different accessibility contracts.

---

## Framework adaptation

| Concept | React / Preact | Vue 3 | Svelte 5 | SolidJS | Leptos |
| --- | --- | --- | --- | --- | --- |
| Click handler | `onClick` | `@click` | `onclick` | `onClick` | `on:click` |
| Class override | `className` | `class` | `class` | `class` | `class` |
| Icon slot | `icon` prop as `ReactNode` | `#icon` slot | `{@render icon()}` | `icon` as `JSX.Element` | `icon` as `Children` |
| Required prop warning | `console.error` in dev | `console.warn` in dev | Svelte prop warning | console in dev | cfg!(debug) panic |

---

## Accessibility requirements

- **Native element.** Always use `<button>`. Never simulate with `<div>` or `<span>` + click handler.
- **`aria-label` is required.** There is no visible text — the label is the only accessible name. Without it, screen readers announce the button as "button" with no description.
- **`aria-busy` when loading.** Set `aria-busy="true"` on root when `loading` is true.
- **`title` attribute.** Set to `tooltip` prop, falling back to `aria-label`. Provides minimal native tooltip. Not a substitute for a styled Tooltip in production.
- **Focus ring.** 2px solid `--color-focus-ring`, visible on all variants. 3:1 contrast minimum against adjacent surface (WCAG SC 1.4.11). Because the component is square, the focus ring is visually clear even at `sm` (28px).
- **Disabled via attribute.** Use the HTML `disabled` attribute — not only CSS class. The attribute removes the element from tab order and announces its state to screen readers.
- **Minimum click target.** `sm` at 28×28px meets the absolute minimum. In contexts where the user is likely on a touch device or using a trackpad, prefer `md` (36×36px) to meet the 32px practical minimum.
- **Danger via icon + context, not color alone.** A trash icon in a clearly labeled "danger zone" section is accessible. A trash icon in an ambiguous context is not — use `Button variant="danger"` with a label instead.

---

## Anti-patterns

**Omitting `aria-label`.** There is no visible label. Without `aria-label`, screen reader users have no information about the action. This is an accessibility failure, not a preference.

**Using `title` without `aria-label`.** `title` alone is not an accessible name in all browsing contexts. Always set `aria-label`; `title` is a fallback display mechanism, not the primary label.

**Using IconButton for actions the icon does not self-evidently communicate.** If you find yourself writing a long `aria-label` to explain what the button does, that is a signal to use `Button` with a visible label instead.

**Using IconButton for primary CTAs.** Primary actions need label text. An icon-only primary button relies entirely on the user recognizing the icon — which is never guaranteed on first encounter.

**Skipping `TooltipIconButton` in production.** Native `title` tooltips are inaccessible on touch devices, delay-rendered, and unstyled. Every IconButton in the shipped product must use the Tier 2 compound.

**Setting width/height on the passed icon.** The component controls icon size via CSS on `> svg`. An icon with hardcoded dimensions will override this and break the size scale.

**Multiple danger IconButtons with no spacing.** Danger buttons must have 24px separation from adjacent actions. In a dense action group, this is achieved with a spacer element, not by reducing the gap between other buttons.

**Ghost IconButton for a destructive action.** `ghost` signals optional/low-emphasis. A delete action needs `danger` variant so it is visually distinct at rest, before any hover interaction.

---

## Implementation checklist

- [ ]  All five variants render with correct token values in light and dark mode
- [ ]  Component is always square: width = height at every size
- [ ]  Icon is centered via flexbox — not positioned with explicit padding
- [ ]  Icon sizes: 14px at sm, 16px at md, 20px at lg
- [ ]  Hover transitions complete in 150ms ease-out
- [ ]  `ghost` hover: background transitions to `surface-2` and color shifts to `text-primary`
- [ ]  Focus ring appears at 0ms on all variants
- [ ]  Active state scale(0.97) on mousedown and keydown (Space / Enter)
- [ ]  Disabled state uses HTML `disabled` attribute, not CSS class alone
- [ ]  Loading state: spinner replaces icon, opacity 0.7, pointer-events suppressed
- [ ]  Spinner sizes: 13px sm, 14px md, 16px lg — 700ms linear
- [ ]  `aria-label` is set on the `<button>` element
- [ ]  `title` attribute is set (tooltip prop or aria-label fallback)
- [ ]  `aria-busy="true"` applied when loading
- [ ]  Runtime warning in development if `aria-label` is absent
- [ ]  `type="button"` is the default
- [ ]  `prefers-reduced-motion` suppresses all transitions and spinner animation
- [ ]  WCAG AA contrast (4.5:1) for icon color on background in both themes
- [ ]  Focus ring contrast: 3:1 against adjacent surface in both themes
- [ ]  No layout shift on loading state change (spinner fills same space as icon)
- [ ]  Danger variant: 24px separation from adjacent non-danger buttons documented and enforced at usage site
- [ ]  `TooltipIconButton` Tier 2 compound exists and wraps this component before shipping