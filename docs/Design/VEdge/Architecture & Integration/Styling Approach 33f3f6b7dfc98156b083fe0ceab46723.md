# Styling Approach

### Tailwind v4 + plain CSS — when to use which

---

## The two-tool rule

This codebase uses **Tailwind v4** and **plain CSS** together. They are not competing tools — each handles a distinct category of work.

**Tailwind handles:** layout, composition, color, and typography — anything that applies design tokens to elements in a predictable, single-value way.

**Plain CSS handles:** behavior — stateful transitions, keyframe animations, multi-property selectors, and any pattern where Tailwind's utility model creates more noise than clarity.

---

## Use Tailwind for

### Layout and composition

```html
<div class="flex items-center gap-2 px-4 py-2">
<div class="grid grid-cols-[220px_1fr] h-screen">
<div class="flex flex-col gap-6 max-w-[65ch]">
```

### Color (via semantic tokens)

After `tokens.css` is imported, every `--color-*` token becomes a Tailwind utility. Dark mode is automatic — Rust injects the correct token values at startup, and the utilities reference `var(--color-*)` which already have the right values. No `dark:` variant needed.

```html
<div class="bg-background text-text-primary border border-border">
<button class="bg-primary text-primary-foreground">
<span class="bg-danger-muted text-danger-text">
```

### Typography

```html
<p class="text-sm font-normal">           body text
<span class="text-sm font-medium">        label
<h2 class="text-xl font-semibold tracking-tight">  section heading
<code class="font-mono text-sm">          password / code
```

### Simple hover and focus

```html
<div class="hover:bg-surface-2 cursor-pointer">
<a class="hover:text-text-primary text-text-secondary">
```

---

## Use plain CSS for

### Component state machines

When a component has multiple states that interact — hover blocked by disabled, active blocked by loading — Tailwind classes become unreadable and brittle.

```css
.btn--primary {
  background: var(--color-primary);
  color: var(--color-primary-foreground);
  transition:
    background-color var(--duration-fast) var(--ease-out),
    transform        var(--duration-micro) var(--ease-out);
}

.btn--primary:hover:not(:disabled):not(.btn--loading) {
  background: var(--color-primary-hover);
}

.btn--primary:active:not(:disabled):not(.btn--loading) {
  transform: scale(0.97);
}

.btn--primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}
```

### Multi-property transitions

```css
/* Correct — plain CSS */
.panel-enter {
  transition:
    transform  var(--duration-base) var(--ease-out),
    opacity    var(--duration-base) var(--ease-out);
}
```

### Keyframe animations

```css
@keyframes btn-spin {
  to { transform: rotate(360deg); }
}

.btn__spinner {
  animation: btn-spin 700ms linear infinite;
}
```

### Focus ring

```css
.btn:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}
```

Tailwind's `focus-visible:ring-2` uses box-shadow, not outline. This can conflict with other box-shadow values. Use `outline` in plain CSS for focus rings.

### CSS variable inheritance tricks

```css
.sidebar-item[aria-selected="true"] {
  --item-text-color: var(--color-text-primary);
}

.sidebar-item {
  --item-text-color: var(--color-text-secondary);
  color: var(--item-text-color);
}
```

---

## Never do

**Hardcode colors anywhere.** `bg-[#2563EB]` in Tailwind and `background: #2563EB` in CSS are equally wrong. Both bypass the token system, break dark mode, and break user theming.

**Use `!important` outside of reduced motion.** The only legitimate use is in `prefers-reduced-motion`.

**Tailwind arbitrary values for design token dimensions.** `w-[220px]` doesn't stay in sync with `--sidebar-width`. Use `var(--sidebar-width)` in plain CSS.

**Mix inline `style=""` with Tailwind classes for visual properties.** Inline styles bypass the utility system and cannot be dark-mode aware.

---

## The decision rule

> "Is this a single design token applied to a single CSS property, with no state dependency?"
> 

If yes → Tailwind utility class.

If no (multiple properties, state selectors, animations, inheritance) → plain CSS file.

---

## File organization

```
src/
  styles/
    tokens.css          ← import first
    components/
      btn.css           ← .btn, .btn--primary, .btn--sm, states
      input.css
      sidebar.css
      toast.css         ← @keyframes
      modal.css         ← @keyframes
```

Tailwind utilities live in component markup files. Plain CSS lives in `styles/components/`. They appear in different files and do not compete.

---

## Quick reference — token → Tailwind utility

| Token | Utility class |
| --- | --- |
| `--color-background` | `bg-background` |
| `--color-surface-1` | `bg-surface-1` |
| `--color-surface-2` | `bg-surface-2` |
| `--color-primary` | `bg-primary` |
| `--color-primary-foreground` | `text-primary-foreground` |
| `--color-danger` | `bg-danger` |
| `--color-text-primary` | `text-text-primary` |
| `--color-text-secondary` | `text-text-secondary` |
| `--color-border` | `border-border` |
| `--font-sans` | `font-sans` |
| `--font-mono` | `font-mono` |
| `--text-sm` (14px/20px) | `text-sm` |
| `--text-xs` (12px/16px) | `text-xs` |
| `--font-weight-medium` | `font-medium` |
| `--font-weight-semibold` | `font-semibold` |
| `--tracking-tight` | `tracking-tight` |
| `--radius` (6px) | `rounded` |
| `--radius-lg` (8px) | `rounded-lg` |
| `--duration-fast` | `duration-fast` |
| `--duration-base` | `duration-base` |
| `--ease-out` | `ease-out` |