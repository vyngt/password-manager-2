# UX/UI Design System — Modern Minimal Tauri App

A clean, minimal, modern desktop application demands a design foundation built on proven UX principles, systematic spacing and color tokens, and deliberate restraint in every visual decision. This reference compiles everything needed to design and build a password manager in Tauri — from cognitive psychology principles to implementation-ready CSS variables. The aesthetic target: **14px body text, 4px spacing grid, a neutral zinc palette, and sub-200ms micro-interactions**.

---

## Part 1 — Foundational UX principles

### Nielsen's 10 usability heuristics applied

1. **Visibility of system status** — Show locked/unlocked state prominently. Display "Password copied — clipboard clears in 30s" toasts.
2. **Match between system and real world** — Use "Vault" not "database," "Master Password" not "auth credential." Strength shown as Weak/Fair/Strong.
3. **User control and freedom** — Soft-delete Trash (recoverable 30 days). Ctrl+L to instantly lock. Always show Cancel alongside Save.
4. **Consistency and standards** — Ctrl+C/V/X, Ctrl+F for search, Ctrl+N for new entry. If it's "Entry" in one place, never "Item" elsewhere.
5. **Error prevention** — Real-time master password validation. Confirm before destructive actions.
6. **Recognition rather than recall** — Favicons next to entries, persistent sidebar, instant search-as-you-type, recently accessed items.
7. **Flexibility and efficiency** — Ctrl+B (copy username), Ctrl+Shift+C (copy password), Ctrl+K (command palette).
8. **Aesthetic and minimalist design** — Main view: entry name, username, favicon only. Passwords behind dots with reveal toggle. Progressive disclosure for advanced fields.
9. **Help users recover from errors** — "Incorrect master password. Please try again." not "AUTH_FAILED_001."
10. **Help and documentation** — Contextual tooltips on icons. 3-4 step first-run onboarding.

### Gestalt principles for layout

**Proximity** — Elements close together are perceived as related. Use a 2:1 ratio between inter-group and intra-group spacing: 8px between label and input, 20–24px between form groups.

**Similarity** — All clickable buttons share the same shape, height, and font weight. All destructive actions consistently use red.

**Figure-ground** — Modal dialogs darken the background with 50–60% opacity. Dropdown menus use `box-shadow` to float. The active sidebar item gets a highlighted background.

**Common region** — Each settings section gets its own card. Password generator controls inside a bordered panel.

### Fitts's Law — sizing targets

| Element | Minimum | Recommended |
| --- | --- | --- |
| Icon-only buttons | 24×24px | 32×32px with padding |
| Standard buttons | 32px tall | **36–44px tall** |
| Primary CTA (Unlock) | 44px tall | **48px, full-width** |
| Vault entry row | — | **48–56px, entire row clickable** |
| Danger vs safe buttons | — | **24px+ separation** |

### Hick's Law — reducing decision time

Limit primary navigation to 5–7 items. Limit context menus to 7±2 items. The command palette (Ctrl+K) is the most powerful Hick's Law countermeasure — it narrows choices with each keystroke. Show "Recently Used" (3–5 items) on the dashboard. Default form shows Name, URL, Username, Password — TOTP, Notes, Custom Fields behind "More fields."

---

## Part 2 — Design systems worth studying

### shadcn/ui — component primitive philosophy

shadcn/ui is a copy-paste component collection built on Radix UI primitives + Tailwind CSS. You own the source code in your project — ideal for a security-focused app.

The theming system uses CSS custom properties — components read `var(--color-primary)` rather than hardcoded values. shadcn/ui uses a `.dark` class to swap its palette; this project does not — theme values are injected by the Rust backend at startup instead (see `color-token-framework.md`).

**Adopt from shadcn/ui:** The CSS variable theming architecture, the Command palette, Dialog for entry forms, and the Zinc neutral palette.

### Apple HIG for desktop-native patterns

Key takeaways: **sidebar + detail two-panel layout** (sidebar 200–260px). Body text at **13px** on macOS. `backdrop-filter: blur(20px)` for sidebar vibrancy. Standard shortcuts: **Cmd+,** for settings, **Cmd+F** for search.

### Microsoft Fluent Design System

**4px spacing grid**, **elevation tokens** (layered shadows), **material effects** via `backdrop-filter`. Motion: 150ms for micro-interactions, 300ms for layout changes, ease-out for entrances.

### Three apps that define modern minimal desktop design

**Linear** — keyboard-first, command-palette-centric. Nearly monochromatic with selective purple accent. Tables have no borders — alignment and spacing create structure. Body text: Inter at 13–14px.

**Vercel dashboard** — stark black-on-white. No gradients, rarely shadows — 1px borders do structural work. Three button variants only: Primary, Secondary, Danger. Border radius: 6px.

**Notion** — content-first design where the UI disappears. Controls appear only on hover. Sidebar resizable and collapsible. Warmer than Linear — primary text `#37352F`, not pure black.

---

## Part 3 — Implementation specifications

### Typography system

Use **Inter** as the primary UI font and **JetBrains Mono** for passwords, keys, and code. Self-host via Fontsource.

**14px is the correct body size for desktop** — not 16px. Font weights: **400** body, **500** labels and nav (primary emphasis tool), **600** headings and buttons, **700** only for critical warnings.

| Token | Size | Line height | Use |
| --- | --- | --- | --- |
| text-xs | 12px | 16px | Captions, timestamps, badges |
| **text-sm** | **14px** | **20px** | **Primary body text** |
| text-base | 16px | 24px | Longer content, descriptions |
| text-lg | 18px | 28px | Section subtitles |
| text-xl | 20px | 28px | Card/modal titles |
| text-2xl | 24px | 32px | Page titles |

### Spacing on the 4px grid

Every spatial value is a multiple of 4px. Critical ratio: **6px** gap between a label and its input, **20px** between form groups, **32px** between page sections.

### Color reference values

All token values are injected by Rust at startup from the active `ThemeConfig` row in the app database. CSS holds neutral fallback values only.

The two themes that ship as defaults:

**Default (light background):** Base #FFFFFF, Panel #F9FAFB, Hover #F3F4F6, Selected #E5E7EB. Text: primary #111827 (15.4:1 AAA), secondary #4B5563 (7.1:1 AA), tertiary #9CA3AF.

**Default (dark background):** Base #09090B (zinc-950), Panel #18181B, Hover #27272A, Selected #3F3F46. Text: primary #FAFAFA, secondary #A1A1AA, tertiary #71717A.

**Semantic colors** — same token structure across all themes, values shift per theme:

- Primary: **#2563EB** (default light) / **#60A5FA** (default dark)
- Danger: **#DC2626** — default across all themes
- Warning: **#D97706** — default across all themes
- Success: **#16A34A** — default across all themes

**Borders:** Default #E5E7EB / #27272A. Focus ring: #2563EB, `outline: 2px solid; outline-offset: 2px`. Border radius: **6px** buttons/inputs, **8px** cards, **12px** modals.

**Light preset — surfaces:** Base #FFFFFF, Sidebar/panel #F9FAFB, Hover #F3F4F6, Selected #E5E7EB

**Light preset — text:** Primary #111827 (15.4:1 AAA), Secondary #4B5563 (7.1:1 AA), Tertiary #9CA3AF (3.0:1 large text only)

**Dark preset — surfaces:** Base #09090B (zinc-950), Panel #18181B, Hover #27272A, Selected #3F3F46

**Dark preset — text:** Primary #FAFAFA, Secondary #A1A1AA, Tertiary #71717A

**Semantic colors** — same token structure across both presets, values shift for dark:

- Primary: **#2563EB** light / **#60A5FA** dark (blue-600 → blue-400)
- Danger: **#DC2626** — same both presets
- Warning: **#D97706** — same both presets
- Success: **#16A34A** — same both presets

### Motion

Desktop animations target 100–200ms. Easing tokens:

```css
--ease-out:    cubic-bezier(0.16, 1, 0.3, 1);    /* Entrances */
--ease-in:     cubic-bezier(0.55, 0, 1, 0.45);   /* Exits */
--ease-in-out: cubic-bezier(0.45, 0, 0.55, 1);   /* Movement */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* Micro-interactions */
```

Only animate `opacity`, `transform`, `box-shadow`, `background-color`, `border-color`. Never animate `width`, `height`, `margin`, `padding`.

---

## Part 4 — Tauri-specific patterns

**Custom titlebar:** `"decorations": false` in `tauri.conf.json`. Add `data-tauri-drag-region` to the titlebar div. Detect OS to place window controls correctly (left on macOS, right on Windows/Linux).

**System theme detection:** Use `getCurrentWindow().theme()` on launch. Read once, save to config. `onThemeChanged()` is never subscribed to — the app does not follow system switches after launch.

**Window vibrancy:** Tauri v2 supports native blur — Vibrancy on macOS, Mica/Acrylic on Windows 11 — via `windowEffects`. Falls back to `surface.1` when unavailable.

**Global shortcuts and system tray:** Register a global hotkey (e.g., Ctrl+Shift+P) via `@tauri-apps/plugin-global-shortcut`. Add a system tray icon via `@tauri-apps/api/tray`.

---

## Conclusion

The core design decisions reduce to a small set of concrete choices: **14px Inter on a 4px grid, a Zinc neutral palette with blue semantic accent, shadcn/ui components with Radix accessibility primitives, and sub-200ms transitions using ease-out curves.**

The most important insight: **minimal design is not about removing features — it's about progressive disclosure.** Every element should earn its screen space, but no capability should be lost. The strongest minimal interfaces feel fast because they show only what's needed at each moment, while keeping everything else one keystroke away.