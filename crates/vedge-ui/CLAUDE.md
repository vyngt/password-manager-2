# CLAUDE.md — vedge-ui

Scope: conventions specific to this crate. The root `CLAUDE.md` covers Leptos fundamentals and the component authoring workflow — read it first. This file layers on top with rules that apply only to the design-system crate.

## i18n: every user-visible string is reactive

**Rule:** any string a user sees — labels, placeholders, hint text, `aria-label`, `aria-live` announcements, status badges, button text rendered *inside* the component — is a `TextProp`, not `&'static str` or `String`.

**Why:** `TextProp` accepts both static literals (`"Done"`) and reactive signals (`Signal::derive(move || t_string!(i18n, key).to_string())`). Consumers using `leptos_i18n` can swap locale at runtime without rebuilding the component. Hardcoded `&'static str` strings freeze the component in one language; plain `String` accepts a value at construction but isn't reactive.

### What counts as user-visible

| String | Prop type |
|---|---|
| `label`, `placeholder`, `hint`, `description` | `TextProp` |
| `aria_label`, `aria_describedby` content | `TextProp` |
| Internal badge / button / status text rendered by the component | `TextProp` (add a prop for it) |
| Error / validation messages *shown by the component* | `TextProp` |
| aria-live announcement templates | `TextProp` (prefix or suffix — see templating below) |
| `id`, `class`, CSS modifier tokens | `&'static str` — **not user-visible** |
| `accept` (MIME pattern), `input_type`, icon tokens | `&'static str` — **not user-visible** |
| Step number, file byte size, progress percentage | Formatted by the component at render — not a prop |

### Prop signature

```rust
#[prop(into, default = TextProp::default())] done_label: TextProp,
```

- `into` lets consumers pass `&'static str`, `String`, or `Signal<String>` — the `TextProp` conversions handle all three.
- Default to `TextProp::default()` (empty signal), and **fall back to an English literal at render time** with the `text_or` helper. This keeps the component usable without explicit i18n wiring, while still supporting locale switching when the consumer provides a reactive signal.

### Render with fallback

Use the shared helper:

```rust
use crate::utils::text::text_or;

view! { <span>{move || text_or(done_label, "Done")}</span> }
```

Call inside a reactive closure (e.g. `{move || …}`, `Signal::derive`) so locale swaps propagate. Never read `prop.get()` in a static context where you want live updates — the subscription is lost.

### Templating (e.g. "Remove {filename}")

The simple prefix/suffix concat works for most languages:

```rust
let aria = format!("{} {}", text_or(remove_label, "Remove"), filename);
```

When word order matters (RTL languages, Japanese `{name}を削除`), document in the component that consumers should pass a **fully-formed** reactive `Signal<String>` rather than relying on the prefix default. Don't build a mini template engine inside the component.

### Dev assertions for required labels

If a string is *always* needed (e.g. an aria-label on an interactive control with no visible text), add a dev assertion that fires when the prop is empty:

```rust
#[cfg(debug_assertions)]
if aria_label.get_untracked().is_empty() {
    web_sys::console::error_1(
        &"MyComponent: `aria_label` is required for accessibility/i18n.".into(),
    );
}
```

Empty TextProp + dev assertion signals "required for i18n"; English fallback signals "optional — English is the default".

### Never

- Don't write `String` props for labels. `String` is not reactive — locale swap won't update what's on screen.
- Don't read `prop.get_untracked()` for display. That reads the current value *without* subscribing — the DOM won't update on locale change.
- Don't hard-code English inside the component body. If you find yourself typing `"Done"` or `"Error"` or `"Remove"` inside `view!`, that's a missing TextProp prop.

## Shared utilities — check `src/utils/` before writing a helper

Before adding a helper function inside a component file, check if one exists:

| Need | Module | Helpers |
|---|---|---|
| Read a `TextProp` with English fallback | `utils::text` | `text_or(prop, fallback)` |
| Format bytes as `"2.4 MB"`, `"840 KB"` | `utils::format` | `format_bytes(u64) -> String` |
| Format a float without trailing `.0` | `utils::format` | `format_float_display(f64) -> String` |
| Generate a DOM id like `dialog-title-{uuid}` | `utils::id` | `id_with_prefix("dialog-title")` |
| Generate a bare UUID string (e.g. list keys) | `utils::id` | `new_uuid()` |

If a helper belongs in a shared util but isn't there yet, **extract it** rather than duplicating. Rule of thumb: if the same function body appears (or nearly appears) in 2 components, it's shared.

What does *not* belong in `utils/`:

- Anything component-specific (e.g. a Slider-only value-to-pixel mapping).
- Abstractions over patterns that look similar but have subtly different requirements across their call sites (e.g. the race-safe open/close version counters in Dialog / Tooltip / Select — each has different timing and cleanup needs; forcing a shared type obscures that).

## CSS class composition

Components build their root class by joining BEM parts with `.join(" ")`:

```rust
let root_cls = [
    "input-root",
    size.input_root_class(),
    status.input_root_class(),
    if disabled { "input-root--disabled" } else { "" },
    class,
].join(" ");
```

This is the established pattern across 28+ components — don't introduce a class-merging abstraction unless there's a concrete need (e.g. class deduplication) that multiple components share.

## Component categories

Place the new component in the category that matches its **role**, not the spec's section path:

| Category | Module | Role |
|---|---|---|
| `foundation/` | `components/foundation.rs` | Atomic display primitives (Avatar, Badge, Button, IconButton, ProgressBar, StepIndicator, Spinner, QRCode, Separator, Kbd) |
| `form/` | `components/form.rs` | Interactive controls that capture user input (Input, Checkbox, Select, FileUpload, …) |
| `feedback/` | `components/feedback.rs` | Transient, overlay, notify-the-user components (Dialog, Toast, Tooltip) |
| `data_display/` | `components/data_display.rs` | Tabular or paged views (DataTable, Pagination) |
| `icon/` | `components/icon.rs` | Custom SVG icons for product-specific marks (not FontAwesome — those come from `icondata`) |

If you're unsure, match against the nearest existing component:

- "Indicates state without capturing input" → foundation.
- "User types / clicks / drags into it" → form.
- "Appears temporarily on top of other UI" → feedback.

## Tokens — extend, don't duplicate

When a new component needs a `Size` or `Status` class, **add a method to the existing enum** in `primitives/tokens.rs`:

```rust
impl Size {
    pub fn my_component_class(&self) -> &'static str {
        match self {
            Size::Sm => "my-component--sm",
            Size::Md => "",      // default — empty class
            Size::Lg => "my-component--lg",
        }
    }
}
```

Only introduce a new enum if the component has a concept no existing one covers (e.g. `BadgeAppearance`, `DialogSize`, `ProgressVariant`).

## Non-goals for this crate

- No CSS-in-Rust / styled-components — styles live in `src/styles/*.css` under `@layer components`.
- No SSR. This crate is CSR-only; `leptos` is imported with the `csr` feature. Don't add `#[cfg(feature = "ssr")]` blocks.
- No global state inside components. Cross-component communication goes through `provide_context` / `expect_context` at well-scoped boundaries (see Dialog and Toast for examples).
- No `Send`/`Sync` bounds on signals holding `web_sys` objects. CSR storage relaxes these — if you hit a bound error with a `Signal<Vec<File>>`-style type, the issue is usually a missing feature on `web-sys` or a misplaced `SyncStorage`, not a need to wrap things in `Arc<Mutex>`.
