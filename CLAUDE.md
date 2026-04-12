# CLAUDE.md

## Project Overview

VEdge is a Tauri desktop app with a Leptos 0.8 (Rust/WASM) frontend and a custom design system (`vedge-ui`).

## Implementing a UI Component from a Spec

Component specs live in `docs/Design/VEdge/Specs/Components/`. When the user asks you to implement a component, **always read the spec first**, then follow every step below. Do not skip the playground.

### Step-by-step checklist

#### 1. Read the spec and identify which category the component belongs to

| Category | Module file | Examples |
|---|---|---|
| `foundation` | `crates/vedge-ui/src/components/foundation.rs` | Button, IconButton, Badge, Separator, Spinner |
| `form` | `crates/vedge-ui/src/components/form.rs` | Input, Checkbox, Select, Toggle, Label, HelperText, NumberInput |
| `feedback` | `crates/vedge-ui/src/components/feedback.rs` | Toast, Tooltip |
| `data_display` | `crates/vedge-ui/src/components/data_display.rs` | Table |

#### 2. Add token methods if needed

**File:** `crates/vedge-ui/src/primitives/tokens.rs`

If the component uses `Size` or `Status` variants, add a class method to the existing enum:

```rust
// In impl Size
pub fn my_component_class(&self) -> &'static str {
    match self {
        Size::Sm => "my-component--sm",
        Size::Md => "",          // Md is the default — no class needed
        Size::Lg => "my-component--lg",
    }
}

// In impl Status
pub fn my_component_class(&self) -> &'static str {
    match self {
        Status::Default => "",   // Default — no class needed
        Status::Error => "my-component--error",
        Status::Success => "my-component--success",
        Status::Warning => "my-component--warning",
    }
}
```

Only add a new enum if the component introduces a concept that no existing enum covers.

#### 3. Create the CSS file

**Location:** `crates/vedge-ui/src/styles/<component-name>.css`

Rules:
- Wrap everything in `@layer components { ... }`
- Use BEM naming: `.component`, `.component--modifier`, `.component__element`
- Use CSS custom properties from `crates/vedge-ui/src/styles/tokens.css` — **never hardcode** colors, fonts, sizes, radii, durations
- Common tokens: `--color-*`, `--text-*`, `--font-*`, `--height-*`, `--radius*`, `--space-*`, `--duration-*`, `--ease-*`
- Disabled state: `opacity: 0.4; cursor: not-allowed; pointer-events: none;`
- Focus ring: `outline: 2px solid var(--color-focus-ring); outline-offset: 2px;`
- Always add `@media (prefers-reduced-motion: reduce)` to zero out transitions/animations
- Status border colors: `--color-danger`, `--color-success`, `--color-warning`
- Status text colors: `--color-danger-text`, `--color-success-text`, `--color-warning-text`

**Then register it** in `crates/vedge-ui/src/styles/index.css`:
```css
@import "./my-component.css";
```

Group with related imports (foundation, form, feedback sections).

#### 4. Create the Rust component

**Location:** `crates/vedge-ui/src/components/<category>/<component_name>.rs`

For simple components, a single file is enough. For complex ones with keyboard handling or sub-types, create a directory:
```
component_name.rs          <- mod file: mod component; pub use component::ComponentName;
component_name/
  component.rs             <- actual component
```

**Component patterns:**

```rust
use crate::primitives::tokens::{Size, Status};
use leptos::prelude::*;

#[component]
pub fn MyComponent(
    // Required props — no attribute
    id: &'static str,
    // Optional with default — #[prop(optional)] or #[prop(optional, default = ...)]
    #[prop(optional)] size: Size,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "")] class: &'static str,
    // Reactive values — #[prop(into)] with Option<Signal<T>> or Signal<T>
    #[prop(into, default = None)] value: Option<Signal<String>>,
    // Callbacks — Option<Callback<T>>
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    // Children slot
    children: Children,
) -> impl IntoView {
    // ...
}
```

Key conventions:
- **State:** `RwSignal<T>` for internal state, merge with external `Signal<T>` via `move || value.map(|s| s.get()).unwrap_or_else(|| internal.get())`
- **CSS classes:** Build as array joined with spaces: `["base", size.class(), status.class(), if disabled { "..." } else { "" }, class].join(" ")`
- **ARIA:** Return `None` for empty string attributes so they don't render: `let attr = if s.is_empty() { None } else { Some(s) };`
- **Icons:** Use `icondata` crate: `use icondata as i;` then `<Icon icon=i::FaCircleCheckSolid />`
  - Error: `i::FaCircleExclamationSolid`
  - Success: `i::FaCircleCheckSolid`
  - Warning: `i::FaTriangleExclamationSolid`
- **Inline SVG:** For simple shapes (plus, minus, checkmark), use inline `<svg>` like Checkbox does — don't pull in a full icon
- **DOM refs:** `let input_ref = NodeRef::<leptos::html::Input>::new();`
- **Keyboard:** `on:keydown=move |ev: web_sys::KeyboardEvent| { ... }` — call `ev.prevent_default()` for handled keys
- **Events:** `on:input:target=handler` for input events, `on:click=handler` for clicks, `on:blur=handler` for blur

#### 5. Register the module

Add to the appropriate category module file (`foundation.rs`, `form.rs`, or `feedback.rs`):

```rust
pub mod my_component;
pub use my_component::*;
```

Keep entries in alphabetical order.

#### 6. Add a playground page (REQUIRED)

Every component **must** have a playground demo. This is not optional.

**a) Create the page file:** `crates/vedge-app/src/pages/playground/pg_<component_name>.rs`

```rust
use leptos::prelude::*;
// import the component from vedge_ui
use super::common::Section;

#[component]
pub fn MyComponentPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"MyComponent"</h1>

            <Section title="Variants">
                // Show all visual variants
            </Section>

            <Section title="Sizes">
                // Show sm / md / lg if applicable
            </Section>

            <Section title="States">
                // Show disabled, error, success, warning, loading if applicable
            </Section>

            <Section title="Interactive">
                // Controlled demo with signals showing live value
            </Section>
        </div>
    }
}
```

Demo guidelines:
- Show **every** prop variant the spec defines
- Include at least one **interactive/controlled** demo with value readout
- Use `Signal::stored(...)` for static demos, `Signal::derive(...)` for reactive ones
- Compose with related components where natural (e.g., Label + Input + HelperText)

**b) Register the module** in `crates/vedge-app/src/pages/playground.rs`:
```rust
mod pg_my_component;
```

**c) Add the route** in `crates/vedge-app/src/pages/playground/routes.rs`:
```rust
use super::pg_my_component::MyComponentPage;
// ...
<Route path=path!("/my-component") view=MyComponentPage />
```

**d) Add to the sidebar** in `crates/vedge-app/src/pages/playground/layout.rs`:

Find the correct `SidebarGroup` (Foundation / Form / Feedback) and add:
```rust
SidebarItem { name: "My Component", path: "/playground/my-component" },
```

Keep items in alphabetical order within each group.

#### 7. Build and verify

```
cargo build -p vedge-ui
cargo build -p vedge-app
```

Both must compile with zero new warnings.

### What the spec sections mean

| Spec section | What to do with it |
|---|---|
| **Purpose** | Understand the use case — don't implement outside this scope |
| **Three ownership boundaries** | Tells you what the component controls vs. what tokens/consumer control |
| **Anatomy** | Defines the DOM structure |
| **Props API** | Defines the Rust component props — follow types and defaults exactly |
| **Token mapping** | Tells you which CSS custom properties to use for each element |
| **States** | Defines visual states — implement each one |
| **Keyboard** | Defines keyboard interactions — implement in `on:keydown` handler |
| **Accessibility** | Defines ARIA attributes, roles, labels — implement all of them |
| **CSS implementation** | Reference CSS — adapt to project conventions (`@layer components`, BEM, tokens) |
| **Do not use** | Edge cases to be aware of, not something to implement |
| **Implementation checklist** | Your final verification checklist — every box must be satisfiable |

### File reference

```
crates/vedge-ui/src/
  primitives/tokens.rs           <- Size, Status, Variant enums + class methods
  styles/tokens.css              <- CSS custom properties (colors, spacing, motion, etc.)
  styles/index.css               <- Master CSS import list
  styles/<component>.css         <- Component CSS (create new files here)
  components/foundation.rs       <- Foundation module exports
  components/form.rs             <- Form module exports
  components/feedback.rs         <- Feedback module exports
  components/<category>/<name>.rs <- Component implementation

crates/vedge-app/src/pages/
  playground.rs                  <- Playground mod declarations
  playground/common.rs           <- Shared Section helper component
  playground/layout.rs           <- Sidebar layout + Light/Dark preset buttons
  playground/routes.rs           <- All playground routes
  playground/pg_<name>.rs        <- Individual component demo pages
```
