# tokens.css — Reference

The master design token file. Import once at the CSS entry point:

```css
@import "tailwindcss";
@import "./tokens.css";
```

**Architecture:** Themes are DB records, not CSS modes. CSS holds neutral fallback values (~50ms before Rust injects). Rust reads the active `ThemeConfig` row at startup and injects all `--color-*` variables on `:root`. Tailwind utilities reference `var(--color-*)` so they pick up whatever Rust injects — no `[data-theme]` selector, no CSS-level switching.

**Theme model:** CSS is theme-blind. `tokens.css` defines fallback values only. Rust reads the saved `ThemeConfig` at startup and injects a `<style>` block setting all `--color-*` variables on `:root`. Tailwind utilities reference `var(--color-*)`, so they pick up whatever Rust injects — no `[data-theme]` selector, no CSS-level switching.

**Why `@theme inline`:** Without `inline`, Tailwind inlines static values into utilities — `.bg-primary { background: #2563EB }` — ignoring Rust injection. With `inline`, utilities reference the CSS variable — `.bg-primary { background: var(--color-primary) }` — so Rust-injected values propagate everywhere.

---

## Primitive layer

Raw color values. Never referenced in components or utilities.

```css
:root {
  --zinc-50:  #FAFAFA;  --zinc-950: #09090B;
  --blue-400: #60A5FA;  --blue-600: #2563EB;  --blue-700: #1D4ED8;
  --red-600:  #DC2626;  --red-700:  #B91C1C;
  --amber-600: #D97706; --amber-700: #B45309;
  --green-600: #16A34A; --green-700: #15803D;
  /* ... full list in tokens.css */
}
```

---

## Semantic layer — colors

All values set under `@theme inline` (light preset fallbacks). Rust overrides on `:root` at runtime.

**Surfaces:**

```
--color-background: #ffffff      elevation 0 — page base
--color-surface-1:  #F9FAFB      elevation 1 — sidebar, card
--color-surface-2:  #F3F4F6      elevation 2 — hover state
--color-surface-3:  #E5E7EB      elevation 3 — selected/active
--color-surface-4:  #D1D5DB      elevation 4 — popover, modal
```

**Primary accent:**

```
--color-primary:            blue-600 (#2563EB)
--color-primary-hover:      blue-700 (#1D4ED8)
--color-primary-muted:      blue-50  (badge bg)
--color-primary-foreground: #ffffff
```

**Text hierarchy:**

```
--color-text-primary:   #111827  (15.4:1 — AAA)
--color-text-secondary: #4B5563  (7.1:1  — AA)
--color-text-tertiary:  #9CA3AF  (3.0:1  — large text only)
```

**Semantic groups** (danger, warning, success all follow the same five-token pattern as primary):

```
--color-{group}:            base color
--color-{group}-hover:      hover state
--color-{group}-muted:      soft tint (badge bg)
--color-{group}-foreground: text on group bg
--color-{group}-text:       group text on neutral surfaces
```

---

## Semantic layer — typography

```css
--font-sans: 'Inter', ui-sans-serif, ...;
--font-mono: 'JetBrains Mono', ui-monospace, ...;

--text-sm:                14px;   /* primary body size — never 16px for desktop */
--text-sm--line-height:   20px;
--text-xs:                12px;
--text-xs--line-height:   16px;
/* ... full scale in tokens.css */

--font-weight-normal:   400;
--font-weight-medium:   500;
--font-weight-semibold: 600;

--tracking-tight:  -0.025em;   /* headings ≥ text-xl */
--tracking-wide:    0.06em;    /* overline / uppercase labels */
```

---

## Semantic layer — motion

```css
--duration-instant:  0ms;    /* focus ring, disabled */
--duration-micro:   80ms;    /* active press/release */
--duration-fast:   150ms;    /* hover, tooltip, dropdown — default */
--duration-base:   200ms;    /* panel entrance, modal open */
--duration-slow:   300ms;    /* toast notification */

--ease-out:    cubic-bezier(0.16, 1, 0.3, 1);     /* entrances */
--ease-in:     cubic-bezier(0.55, 0, 1, 0.45);    /* exits */
--ease-in-out: cubic-bezier(0.45, 0, 0.55, 1);    /* repositioning */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* micro-interactions only */
```

---

## Component sizing

```css
--height-sm:       28px;   /* sm button/input */
--height-md:       36px;   /* default */
--height-lg:       44px;   /* primary CTA */
--height-entry:    48px;   /* vault entry row */
--height-entry-lg: 56px;   /* entry row with subtitle */
--sidebar-width:   220px;

--radius-sm:    4px;   /* chips */
--radius:       6px;   /* buttons, inputs — default */
--radius-lg:    8px;   /* cards, panels */
--radius-xl:   12px;   /* modals */
```

---

## Shadows

Fallback values — Rust overrides at startup based on active theme.

```css
/* Light preset (drop shadows) */
--shadow-sm:    0 1px 3px rgba(0, 0, 0, 0.08);
--shadow-md:    0 4px 12px rgba(0, 0, 0, 0.12);
--shadow-lg:    0 8px 32px rgba(0, 0, 0, 0.16);
--shadow-scrim: rgba(0, 0, 0, 0.50);

/* Dark preset (glow borders — Rust injects these) */
--shadow-md:    0 0 0 1px rgba(255, 255, 255, 0.10);
--shadow-lg:    0 0 0 1px rgba(255, 255, 255, 0.14);
--shadow-scrim: rgba(0, 0, 0, 0.70);
```

---

## Spacing scale

Plain CSS custom properties only — not in `@theme`. Use Tailwind's default spacing scale for utility classes (they align perfectly with the 4px grid). Use `--space-*` tokens in plain CSS component files.

```css
--space-0_5:  2px;   --space-1:    4px;   --space-1_5:  6px;
--space-2:    8px;   --space-3:   12px;   --space-4:   16px;
--space-5:   20px;   --space-6:   24px;   --space-8:   32px;
--space-10:  40px;   --space-12:  48px;   --space-16:  64px;
```