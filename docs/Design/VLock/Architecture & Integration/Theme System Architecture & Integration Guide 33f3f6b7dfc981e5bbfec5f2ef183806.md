# Theme System Architecture & Integration Guide

This document answers the question **"how do I wire the token system into a new project?"** — the layer boundaries, what the UI library exports, what the application implements, and what is never permitted.

This is an **Architecture Guide** (sometimes called an Integration Contract). It is not a spec (which describes values) and not a conceptual doc (which describes philosophy). It describes the seams between layers and the API that crosses those seams.

> Read this before setting up the theme system in any new project. Everything here is required — none of it is optional.
> 

---

## The three-layer model

Every part of the theme system belongs to exactly one layer. Crossing layers in the wrong direction causes theming bugs that are hard to trace.

```
┌─────────────────────────────────────────────┐
│             Application Layer               │
│  Business logic, pages, feature components  │
│  Consumes tokens via Tailwind utilities      │
│  Implements ThemeProvider interface          │
│  Calls Rust via invoke() to save/apply       │
├─────────────────────────────────────────────┤
│             UI Library Layer                │  ← token system lives here
│  tokens.css — all --color-*, --duration-*   │
│  Base components (Button, Input, Card...)   │
│  ThemeProvider interface definition         │
│  Validation logic, deriveTokens() utility   │
├─────────────────────────────────────────────┤
│             Platform Layer                  │
│  Rust backend                               │
│  Reads ThemeConfig from app database        │
│  Injects <style> block on :root at startup  │
└─────────────────────────────────────────────┘
```

Tokens flow **downward only**. Application never defines tokens. UI Library never calls Rust. Rust never knows about components.

---

## Why tokens belong to UI Library, not Application

`tokens.css` contains no domain knowledge. It does not know what a "vault entry" or a "master password" is. It only defines `--color-primary`, `--height-md`, `--duration-fast` — values reusable across any app built with this system.

If you copy `tokens.css` to a completely different project tomorrow, it works without changes. That is the test: if a file references your app's domain, it does not belong in the UI library.

---

## What the UI Library exports

The UI library defines the **contract**. Application provides the **implementation** of that contract.

### Types

```tsx
// The user-configurable inputs — what the Settings UI collects
export interface ThemeConfig {
  background: string    // hex
  foreground: string    // hex
  primary:    string    // hex
  // danger/warning/success intentionally omitted — not user-configurable
}

// The full derived token set — read-only, computed by Rust from ThemeConfig
export interface ThemeTokens {
  colorBackground:        string
  colorSurface1:          string
  colorSurface2:          string
  colorSurface3:          string
  colorSurface4:          string
  colorPrimary:           string
  colorPrimaryHover:      string
  colorPrimaryMuted:      string
  colorPrimaryForeground: string  // computed via contrast math, never manual
  colorTextPrimary:       string
  colorTextSecondary:     string
  colorTextTertiary:      string
  colorBorder:            string
  colorBorderStrong:      string
  colorDanger:            string
  colorWarning:           string
  colorSuccess:           string
  colorFocusRing:         string
}

// Validation result for a single contrast check
export interface ContrastResult {
  ratio:  number
  pass:   'AAA' | 'AA' | 'fail'
  label:  string   // e.g. "Text on primary button: 5.2 : 1 ✓"
}

// Full validation report for a ThemeConfig
export interface ThemeValidation {
  bodyText:    ContrastResult
  primaryText: ContrastResult
  focusRing:   ContrastResult
  isValid:     boolean   // false = block save action
}
```

### Provider interface

```tsx
export interface ThemeProvider {
  // Current active token set (reactive — updates when theme changes)
  current: ThemeTokens

  // Validate a config before saving — used for live preview in Settings
  validate(config: ThemeConfig): ThemeValidation

  // Apply a visual preview to :root without saving to DB
  preview(config: ThemeConfig): void

  // Commit to DB — UI Library defines the shape; Application implements the body
  commit(config: ThemeConfig): Promise<void>

  // Cancel preview, revert :root to current saved theme
  revert(): void
}
```

### Pure utilities (no provider needed)

```tsx
// Derive full token set from a raw ThemeConfig
export function deriveTokens(config: ThemeConfig): ThemeTokens

// Validate contrast ratios against WCAG thresholds
export function validateThemeConfig(config: ThemeConfig): ThemeValidation

// Compute accessible foreground color for primary button text
export function computePrimaryForeground(primary: string): '#ffffff' | '#111827'

// Inject a token set onto :root as CSS custom properties
export function injectCSSVars(tokens: ThemeTokens): void
```

### Context + components

```tsx
export { ThemeContext }    // React/Svelte/Vue context — Application registers the impl
export { ColorPicker }    // Input for hex color + live contrast display
export { ContrastBadge }  // Renders a ContrastResult as a visual badge
```

---

## What the Application implements

Application provides the `commit()` body and registers the provider. Everything else it receives from the UI Library.

```tsx
// app/src/theme/rustThemeProvider.ts
import type { ThemeProvider, ThemeConfig, ThemeTokens } from '@your-org/ui'
import { deriveTokens, validateThemeConfig, injectCSSVars } from '@your-org/ui'
import { invoke } from '@tauri-apps/api/core'

function getCurrentTokensFromDOM(): ThemeTokens {
  const style = getComputedStyle(document.documentElement)
  return {
    colorBackground: style.getPropertyValue('--color-background').trim(),
    colorPrimary:    style.getPropertyValue('--color-primary').trim(),
    // ... read all --color-* from :root
  }
}

export const rustThemeProvider: ThemeProvider = {
  current: getCurrentTokensFromDOM(),

  validate(config) {
    // Pure function from UI Library — no Rust needed
    return validateThemeConfig(config)
  },

  preview(config) {
    // Inject temporarily — not saved
    injectCSSVars(deriveTokens(config))
  },

  async commit(config) {
    // Application knows how to talk to Rust — UI Library does not
    await invoke('save_theme', { config })
  },

  revert() {
    // Ask Rust to reinject the saved theme
    invoke('apply_active_theme')
  }
}
```

```tsx
// app/src/main.tsx
import { ThemeContext } from '@your-org/ui'
import { rustThemeProvider } from './theme/rustThemeProvider'

<ThemeContext.Provider value={rustThemeProvider}>
  <App />
</ThemeContext.Provider>
```

---

## What Application must never do

These patterns bypass the system and break theming. They are not permitted even if they "work" locally.

```tsx
// ❌ Inject CSS vars directly — bypasses validate(), breaks preview/revert
document.documentElement.style.setProperty('--color-primary', '#ff0000')

// ❌ Call Rust without validating — low-contrast theme can be saved
await invoke('save_theme', { config: unvalidatedConfig })

// ❌ Derive tokens independently — logic must live in UI Library
const hover = darken(primary, 10)

// ❌ Hardcode a color anywhere in component markup or CSS
<button class="bg-[#2563EB]">          // Tailwind arbitrary value
background: #2563EB;                   // Plain CSS

// ❌ Read a primitive token in a component
.btn { background: var(--blue-600); }  // Primitive — bypasses semantic layer
```

---

## Token flow summary

```
User picks color in Settings
        ↓
validate(config)          ← UI Library pure function
        ↓ if valid
preview(config)           ← injectCSSVars(deriveTokens(config)) on :root
        ↓ user confirms
commit(config)            ← Application calls invoke('save_theme')
        ↓
Rust saves to DB, injects <style> block on :root
        ↓
Tailwind utilities update automatically
(bg-primary → var(--color-primary) → new value)
```

---

## Checklist — setting up in a new project

- [ ]  Import `tokens.css` as the first CSS import, before Tailwind
- [ ]  Confirm `@theme inline` is present so Tailwind utilities reference `var()` not static values
- [ ]  Implement `ThemeProvider` interface in the application layer (the `rustThemeProvider` pattern above)
- [ ]  Register `ThemeContext.Provider` at the app root with the implementation
- [ ]  Wire `commit()` to the Rust `save_theme` command
- [ ]  Wire `revert()` to `apply_active_theme`
- [ ]  Confirm Rust calls `inject_theme()` on startup before the window is visible
- [ ]  Set the window background color in `tauri.conf.json` to match `--color-background` fallback to prevent white flash
- [ ]  Verify no component references a primitive token (`--zinc-*`, `--blue-*`, raw hex)
- [ ]  Run contrast validation on both default light and dark presets before shipping

---

## File locations

```
packages/
  ui/                          ← UI Library — owns all of this
    tokens.css
    src/
      theme/
        index.ts               ← exports all types + utilities + context
        derive.ts              ← deriveTokens(), computePrimaryForeground()
        validate.ts            ← validateThemeConfig(), ContrastResult logic
        inject.ts              ← injectCSSVars()
      components/
        ColorPicker.tsx
        ContrastBadge.tsx

  app/                         ← Application — implements the contract
    src/
      theme/
        rustThemeProvider.ts   ← ThemeProvider impl with invoke()
      main.tsx                 ← ThemeContext.Provider registration
```