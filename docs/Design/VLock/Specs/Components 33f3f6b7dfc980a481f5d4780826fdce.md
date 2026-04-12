# Components

## What are component tiers?

The component system is organized into four tiers. Each tier builds on the one below it. Tier 1 components are completely self-contained — they depend only on the token system. Tier 4 components are complete screens assembled from everything below.

**The one hard rule:** no tier may import from a higher tier. Tier 2 composes Tier 1. Tier 3 assembles Tier 2. Tier 4 instantiates Tier 3. A violation of this direction is always a sign that a component has been placed in the wrong tier.

---

## Tier 1 — Atomic

The smallest indivisible units. Each Tier 1 component reads CSS tokens directly, manages its own visual states, and renders without children that are themselves components.

**18 components** across 5 categories: Button, Icon Button, Toggle, Checkbox, Select, Input, Label, Helper Text, Password Strength Meter, Textarea, Badge, Avatar, Separator, Spinner, Kbd, Tooltip, Toast, Focus Ring.

[Tier 1 — Atomic](Components/Tier%201%20%E2%80%94%20Atomic%2033f3f6b7dfc980dfa2a7dbb1828c120b.md)

---

## Tier 2 — Composite

Components built by composing two or more Tier 1 pieces into a unit with a single bounded purpose. Examples: Form Field (Label + Input + Helper Text), Dialog, Command Palette.

**9 components:** Form Field, Dialog, Dropdown Menu, Context Menu, Popover, Command Palette, Sidebar Item, Entry Row, Toast Group.

[Tier 2 — Composite](Components/Tier%202%20%E2%80%94%20Composite%2033f3f6b7dfc981659629eef3372ec253.md)

---

## Tier 3 — Pattern

Recurring structural layouts specific to this application. A Pattern defines how Tier 2 components relate spatially within a fixed interaction context. Patterns know application structure but not application state.

**8 patterns:** Sidebar Navigation, Vault Entry List, Entry Detail Panel, Two-panel Shell, Settings Section, Unlock Screen, Empty State, Command Overlay.

[Tier 3 — Pattern](Components/Tier%203%20%E2%80%94%20Pattern%2033f3f6b7dfc981c8963ac030848fb64b.md)

---

## Tier 4 — Page

Complete application screens. Each page instantiates one or more Tier 3 Patterns and is the only tier aware of routing, global state, and application-level context.

**7 pages:** Unlock Screen, Main Vault View, Entry Detail View, New Entry Form, Generator, Settings Page, Onboarding Flow.

[Tier 4 — Page](Components/Tier%204%20%E2%80%94%20Page%2033f3f6b7dfc9816cba41d2e68419e759.md)