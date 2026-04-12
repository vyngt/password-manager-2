# Tier 2 — Composite

## Overview

Tier 2 components are built by composing two or more Tier 1 pieces into a component with a single, bounded purpose. A Tier 2 component is the smallest unit that requires child components to function.

**Rule:** No Tier 2 component imports another Tier 2 component. If that becomes necessary, the result is Tier 3.

---

## Component list

| Component | Tier 1 pieces used | Purpose |
| --- | --- | --- |
| Form Field | Label + Input + Helper Text | Complete labeled input with validation feedback |
| Dialog / Modal | Surface + Scrim + Focus Ring | Blocking overlay for confirmations and forms |
| Dropdown Menu | Select trigger + Popover list + Separator | Floating action list anchored to a trigger |
| Context Menu | Same as Dropdown, pointer-triggered | Right-click action list |
| Popover | Surface + Focus Ring | Non-blocking floating panel |
| Command Palette | Input + Dropdown list + Kbd hints | Ctrl+K search-first navigation overlay |
| Sidebar Item | Avatar + Badge + Icon + Label | Single navigable row in the sidebar |
| Entry Row | Avatar + Badge + Label + Icon Button | Single vault entry in the list view |
| Toast Group | Toast (managed stack) | Stacked notification queue, top-right |

---

*Specs to be added per component.*