# CLAUDE.md

## Project Overview

VEdge is a Tauri desktop app with a Leptos 0.8 (Rust/WASM) frontend and a custom design system (`vedge-ui`). Target is CSR (Client-Side Rendering) only — SSR / cargo-leptos are **not** in scope.

## Leptos fundamentals

Full reference: [`docs/Leptos Development Guidelines — Rust 2024 Edition.md`](docs/Leptos%20Development%20Guidelines%20%E2%80%94%20Rust%202024%20Edition.md). Read that first when the task touches reactivity, async, or global state. The rules below are the subset that matters for component authoring in `vedge-ui`.

### Signal hierarchy — pick the least powerful thing that works

| Shape | Use when |
|---|---|
| `move \|\| expr` (plain closure) | Default for derived values — zero allocation, recomputes on read |
| `signal(T)` → `(ReadSignal, WriteSignal)` | Local component state with clear read/write separation |
| `RwSignal::new(T)` | Internal state that must be captured by several closures or stored in a struct (`Copy + 'static`) |
| `Memo::new(\|_\| expr)` | Expensive computation **or** to suppress downstream re-runs when the value hasn't changed (uses `PartialEq`) |
| `StoredValue::new(T)` | Non-reactive captured values: configs, heavy structs, `NodeRef` groups |

Reach for `Memo` only when the work is non-trivial or you need to gate re-renders — additions and lookups belong in a plain closure.

### Reactivity anti-patterns

- **Static snapshot**: `view! { <p>{count.get()}</p> }` renders once. Use `{count}` (signals are `IntoView`) or `{move || count.get()}`.
- **Derive via Effect**: don't `set_derived.set(...)` inside an `Effect` — use a closure or `Memo`. `Effect` is only for side effects leaving the reactive world.
- **`<For>` keyed by index**: always key by a stable unique ID. Reorders otherwise produce wrong DOM.
- **Branch type mismatch in `view!`**: two `view!` arms in an `if/else` don't type-check. Use `leptos::either::{Either, EitherOf3, EitherOf4}` or, for boolean toggles, `<Show when=… fallback=…>`.
- **Inline `{move || if cond { view!{…A} } else { view!{…B} }}`**: the closure re-runs whenever any signal inside it changes. `<Show>` memoizes the predicate — prefer it for non-trivial branches.
- **`RwSignal<Vec<RwSignal<T>>>`**: inner signals leak when the outer `Vec` shrinks. Use `#[derive(Store)]` from `reactive_stores` for structured/nested app state.

### Types and ownership in props

- **Literal-only props** (`id`, `class`, icon tokens): `&'static str` is fine — `'static` doesn't trigger Rust 2024's RPIT capture rules.
- **User-provided strings** (labels, placeholders): prefer `String`, or `Signal<String>` for reactive content. Do **not** introduce `&'a` lifetime parameters on `#[component]` functions — they collide with 2024 RPIT capture.
- **Cross-boundary integers** (IDs, counts): use explicit widths (`u32`, `u64`). WASM is 32-bit, so `usize` values above `u32::MAX` silently truncate.
- **Callbacks**: `Callback<T>` (not bare `Fn`) — it's `Copy + 'static` and plays well with `#[prop(into)]`.

### Controlled inputs

Never write `value=signal` on an `<input>` — that sets the HTML attribute once and then stops. Use the DOM property binding:

```rust
view! {
    <input
        prop:value=move || name.get()
        on:input:target=move |ev| set_name.set(ev.target().value())
    />
}
```

### Dynamic attributes and classes

```rust
view! {
    <button
        class="btn"
        class:active=move || is_active.get()                  // toggle a single class
        class=("btn--primary", move || variant.get().is_primary()) // class name w/ special chars
        style:opacity=move || if loading.get() { "0.5" } else { "1" }
        aria-pressed=move || is_pressed.get().then_some("true") // Option<&str> → renders or skipped
        prop:value=move || input_value.get()                   // DOM property, not attribute
        on:input:target=move |ev| set_input_value.set(ev.target().value())
    />
}
```

`Option<&str>` is the idiomatic way to suppress an attribute: return `None` and nothing renders.

### Control flow

```rust
// Boolean toggle
<Show when=move || is_open.get() fallback=|| view!{<Closed/>}>
    <OpenPanel/>
</Show>

// Multi-branch
use leptos::either::EitherOf3;
{move || match status.get() {
    Status::Loading => EitherOf3::A(view!{<Spinner/>}),
    Status::Error   => EitherOf3::B(view!{<ErrorView/>}),
    Status::Done    => EitherOf3::C(view!{<Content/>}),
}}

// List — stable unique key, never index
<For
    each=move || items.get()
    key=|item| item.id
    children=|item| view!{ <Row item=item/> }
/>
```

### Async and side effects

- Mutations / commands: `Action::new(|input: &T| async { … })`. Read `.pending()`, `.value()` reactively.
- Data fetch: `Resource::new(source, fetcher)`; browser-only / `!Send` fetches use `LocalResource`. Read inside `<Suspense>` / `<Transition>`.
- No `tokio::spawn` in WASM — use `leptos::task::spawn_local`.
- Tauri calls live in `bridge/` wrappers, not inside components.

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

Key conventions (aligned with the Leptos guide):
- **Controlled/uncontrolled merge:** `RwSignal<T>` for the internal fallback; when computing the effective value, use a plain closure — `let effective = move || value.map(|s| s.get()).unwrap_or_else(|| internal.get());` — or wrap in `Memo::new` only if downstream closures should skip work when the value is unchanged.
- **Controlled `<input>`s:** `prop:value=` (DOM property) plus an `on:input:target=` handler. Never `value=signal` — that sets the HTML attribute once and then detaches.
- **CSS classes:** Build as an array joined with spaces: `["base", size.class(), status.class(), if disabled { "timepicker-root--disabled" } else { "" }, class].join(" ")`. Prefer the `class:name=move || cond` syntax for a single toggled class, or `class=("name-with-dashes", move || cond)` when the class name needs special characters.
- **Reactive attributes:** return `Option<&str>` so the attribute is omitted when empty — `let attr = if s.is_empty() { None } else { Some(s) };`. For boolean-ish ARIA, use `move || is_on.get().then_some("true")`.
- **Callbacks:** accept `Option<Callback<T>>` (or `Callback<T>` when always required). Don't accept bare `Fn`/`FnMut` — `Callback<T>` is `Copy + 'static` and composes with `#[prop(into)]`.
- **Icons:** `use icondata as i;` then `<Icon icon=i::FaCircleCheckSolid />`. Status defaults: Error → `i::FaCircleExclamationSolid`, Success → `i::FaCircleCheckSolid`, Warning → `i::FaTriangleExclamationSolid`.
- **Inline SVG** for simple shapes (plus, minus, checkmark) — see Checkbox. Don't pull in a full icon for two paths.
- **DOM refs:** `let input_ref = NodeRef::<leptos::html::Input>::new();`. To programmatically focus a `Span`, coerce: `let raw: &web_sys::HtmlElement = &el; let _ = raw.focus();`.
- **Event typing:** `on:keydown=move |ev: web_sys::KeyboardEvent| { … }`, `on:input:target=…`, `on:click=move |_: web_sys::MouseEvent| …`, `on:blur=move |_: web_sys::FocusEvent| …`. Call `ev.prevent_default()` for any key you handle.
- **Control flow inside `view!`:** boolean → `<Show when= fallback=>`; list → `<For each= key= children=>` with a **stable unique ID**; multi-branch → `leptos::either::{Either, EitherOf3, …}`. Never branch with raw `if`/`match` returning different `view!` types, and never key `<For>` by index.
- **No derive-via-Effect:** do not set a signal from inside `Effect::new` to keep another signal in sync. Use a closure or `Memo` instead. `Effect` is reserved for side effects that leave the reactive system (console logs, `web_sys` calls, imperative DOM work).

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
