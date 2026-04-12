# Typography & Spacing Framework

### A principled system for readable, consistent, harmonious UI layout

---

## Part 1 — Typography

### The core constraint: desktop is not web

Desktop applications use smaller type than websites. **14px is the correct body size for desktop UI**. VS Code, Linear, Figma, 1Password, and every mature desktop application land between 13–14px for body text. 16px belongs to long-form reading, not UI chrome.

### The type scale

| Token | Size | Line height | Use |
| --- | --- | --- | --- |
| `text-xs` | 12px | 16px (1.33) | Captions, timestamps, badges |
| **`text-sm`** | **14px** | **20px (1.43)** | **Primary body text** |
| `text-base` | 16px | 24px (1.50) | Descriptions, longer prose |
| `text-lg` | 18px | 28px (1.56) | Section subtitles, card titles |
| `text-xl` | 20px | 28px (1.40) | Modal titles, page subtitles |
| `text-2xl` | 24px | 32px (1.33) | Page titles, empty state headings |

### Font weight

Three weights only. More than three creates visual noise.

```
400  regular    body text, field values, descriptions
500  medium     labels, nav items, sidebar items, table headers
600  semibold   headings, button text, emphasis
```

**500 medium is the primary emphasis tool in minimal design**, not 600. When in doubt, reach for 500 before 600. Weight 700 is reserved for critical destructive warnings only.

### Letter spacing

```
tracking-tight   -0.025em    headings at text-xl and above
tracking-normal   0          everything else
tracking-wide     0.06em     overline / uppercase labels only
```

### Font families

```
font-sans    Inter (primary)      all UI text
font-mono    JetBrains Mono       passwords, keys, tokens, code values
```

Monospace for passwords is a functional requirement — fixed-width characters make character-by-character reading possible and prevent layout shift when passwords are revealed. Self-host via Fontsource (Tauri has no guaranteed network access).

### Semantic text roles

Components reference roles, not raw size/weight pairs.

| Role | Size | Weight | Color | Notes |
| --- | --- | --- | --- | --- |
| `page-title` | text-2xl | semibold | primary | tracking-tight |
| `section-title` | text-xl | semibold | primary | tracking-tight |
| `card-title` | text-lg | medium | primary | — |
| `body` | text-sm | regular | primary | — |
| `label` | text-sm | medium | secondary | — |
| `caption` | text-xs | regular | secondary | — |
| `overline` | text-xs | medium | tertiary | uppercase, tracking-wide |
| `code` | text-sm | regular | primary | font-mono |
| `password` | text-sm | regular | primary | font-mono, tracking-wide |

### The three-tier text color system

```
text.primary      body text, headings, entry names, field values
text.secondary    descriptions, labels, sidebar items (inactive)
text.tertiary     placeholders, timestamps, captions, overlines
```

### Maximum line width

Body text must not exceed **65ch** (~520px at 14px). Applies to descriptions and notes fields. Single-line elements use `text-overflow: ellipsis`, not wrapping.

---

## Part 2 — Spacing

### The single rule: everything is a multiple of 4

Every margin, padding, gap, width, and height is a multiple of 4px. No exceptions.

### The scale

```
--space-0_5:  2px    fine adjustment, icon optical alignment
--space-1:    4px    icon-to-text gap (tight)
--space-1_5:  6px    label-to-input gap
--space-2:    8px    component internal padding
--space-3:   12px    comfortable component padding
--space-4:   16px    card padding, form field gap
--space-5:   20px    between form groups
--space-6:   24px    section padding
--space-8:   32px    page padding, major section gap
--space-10:  40px    large section break
--space-12:  48px    page-level vertical spacing
--space-16:  64px    maximum spacing
```

### Component sizing

Heights are not chosen — they derive from the typography system.

```
height = line-height(font-size) + (2 × padding-y)

sm:  28px = 16px + (2 × 6px)
md:  36px = 20px + (2 × 8px)   ← default
lg:  44px = 24px + (2 × 10px)

height-entry:    48px    vault entry row (entire row is click target)
height-entry-lg: 56px    entry row with subtitle visible
```

### Component padding rules

```
button-sm      padding: 0 10px
button-md      padding: 0 16px
button-lg      padding: 0 20px

input-md       padding: 0 12px
card           padding: 16px 20px
card-compact   padding: 12px 16px
sidebar-item   padding: 6px 12px
modal          padding: 24px
```

### The critical ratios

Three ratios appear repeatedly. Memorizing them eliminates most spacing decisions:

- **6px / 20px** — label-to-input (6px), gap between form groups (20px)
- **8px / 24px** — icon-to-text inside component (8px), gap between component groups (24px)
- **16px / 48px** — card internal padding (16px), page-level section gap (48px)

The consistent 3:1 ratio between levels is the structural expression of Gestalt proximity.

### Sidebar spacing

```
sidebar width          220px
sidebar item height    32px
sidebar item padding   6px 12px
sidebar item gap       2px
sidebar group gap      16px
```

2px gap between items, 16px between sections = 8:1 ratio. Section breaks are unmistakable without visible dividers.

---

## Part 3 — How typography and spacing compose

### The composition rule

`height-md` is not chosen — it falls out of the type system:

```
height-md = line-height(text-sm) + (2 × padding-y)
36px      = 20px                 + (2 × 8px)
```

### Form layout composition

```
[Label]               ← role.label, color text.secondary
  ↕ 6px               ← label → input (intra-field)
[Input field]
  ↕ 20px              ← field → next field (inter-field)
[Label]
[Input field]
  ↕ 32px              ← field group → next section
[Section title]
```

---

## Part 4 — Token definitions and Tailwind v4 integration

All tokens live in `tokens.css` and are applied via `@theme inline` (for Tailwind utility generation) and `:root` (for plain CSS consumption).

### Typography tokens — canonical names

```css
/* in tokens.css → @theme inline */

--font-sans: 'Inter', ui-sans-serif, ...;
--font-mono: 'JetBrains Mono', ui-monospace, ...;

/* Font size + line-height (Tailwind v4 paired syntax) */
--text-xs:                12px;
--text-xs--line-height:   16px;
--text-sm:                14px;
--text-sm--line-height:   20px;
--text-base:              16px;
--text-base--line-height: 24px;
--text-lg:                18px;
--text-lg--line-height:   28px;
--text-xl:                20px;
--text-xl--line-height:   28px;
--text-2xl:               24px;
--text-2xl--line-height:  32px;

/* Weights */
--font-weight-normal:   400;
--font-weight-medium:   500;
--font-weight-semibold: 600;

/* Letter spacing */
--tracking-tight:  -0.025em;
--tracking-normal:  0em;
--tracking-wide:    0.06em;
```

### Tailwind typography utilities

```html
<p class="text-sm">            14px / 20px — body text
<p class="text-xs">            12px / 16px — captions
<h2 class="text-xl">           20px / 28px — section title
<p class="font-normal">        400 — body, values
<span class="font-medium">     500 — labels, nav items
<h1 class="font-semibold">     600 — headings, button text
<h1 class="tracking-tight">    headings ≥ text-xl only
<span class="tracking-wide uppercase"> overlines
<p class="font-sans">          Inter
<code class="font-mono">       JetBrains Mono
```

### Spacing tokens — canonical names

```css
/* in tokens.css → :root (plain CSS use only) */

--space-0_5:  2px;
--space-1:    4px;
--space-1_5:  6px;
--space-2:    8px;
--space-3:   12px;
--space-4:   16px;
--space-5:   20px;
--space-6:   24px;
--space-8:   32px;
--space-10:  40px;
--space-12:  48px;
--space-16:  64px;
```

**Tailwind spacing:** Tailwind's default scale aligns with the 4px grid — `p-4` = 16px = `--space-4`, `gap-3` = 12px = `--space-3`. Use Tailwind utilities for layout in markup. Use `--space-*` tokens in plain CSS component files.

### Component sizing tokens

```css
/* in tokens.css → @theme inline */

--height-sm:       28px;
--height-md:       36px;
--height-lg:       44px;
--height-entry:    48px;
--height-entry-lg: 56px;
--sidebar-width:   220px;
```

---

## Summary of rules

1. Body text is 14px. Not 16px, not 13px. 14px.
2. Three font weights only: 400, 500, 600. Weight 700 is reserved for critical warnings.
3. Components reference semantic roles, not raw size/weight pairs.
4. Text hierarchy uses color tiers, not opacity.
5. Every spacing value is a multiple of 4px. No exceptions.
6. Component heights are derived from line-height plus padding — not chosen independently.
7. Three spacing relationship levels: intra-component (tight), inter-component (moderate), section (generous), with roughly 3:1 ratio between levels.
8. Sidebar items are denser than other components by design — density serves navigation.
9. Do not use margin for layout gaps. Use `gap` in flex/grid, `padding` for internal space.
10. Font size is never hardcoded in component styles. It always comes from a scale token via a semantic role.