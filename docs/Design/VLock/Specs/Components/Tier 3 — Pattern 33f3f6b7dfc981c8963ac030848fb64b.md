# Tier 3 — Pattern

## Overview

Tier 3 Patterns are recurring structural layouts specific to this application. A Pattern defines how Tier 2 components relate spatially and behave together within a fixed interaction context. Patterns are not reusable across different applications — they are purpose-built for the vault UI.

**Rule:** Patterns are aware of application structure (sidebar, detail panel, modal layer) but not of application state (which entry is selected, whether the vault is locked).

---

## Pattern list

| Pattern | Components used | Purpose |
| --- | --- | --- |
| Sidebar Navigation | Sidebar Item × n + Separator + Overline label | Left-rail navigation with grouped sections |
| Vault Entry List | Entry Row × n + Empty State | Scrollable list of vault entries |
| Entry Detail Panel | Card + Form Field × n + Button group | Slide-in panel showing full entry details |
| Two-panel Shell | Sidebar + Main content area | The core app layout: sidebar left, content right |
| Settings Section | Card + Form Field + Toggle + Separator | Grouped settings block with section title |
| Unlock Screen | Input + Button (full-width) + Logo | The vault lock screen, Z-pattern layout |
| Empty State | Icon (large) + Heading + Body + Button | Zero-state for empty lists and first-run |
| Command Overlay | Command Palette + Scrim | Full-screen Ctrl+K overlay |

---

*Specs to be added per pattern.*