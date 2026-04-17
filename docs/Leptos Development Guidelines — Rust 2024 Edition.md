# Leptos Development Guidelines — Rust 2024 Edition

> **Status:** Living Document · **Version:** 1.1 · **Layer:** Frontend Engineering
> 

> Guidelines for developing with Leptos 0.8 + Rust 2024 edition in `vedge-app` (CSR/WASM). SSR is not a current target — SSR-specific notes are clearly labeled. **Read the Anti-Patterns section first — avoiding mistakes is the fastest path to writing correct code.**
> 

---

# Quick reference

```
Leptos     0.8.x  (current: 0.8.17)
Rust       ≥ 1.85  (edition = "2024")
Target     wasm32-unknown-unknown
Mode       CSR (Client-Side Rendering) — Tauri WebView
Build      Trunk  (not cargo-leptos — CSR uses Trunk)
Formatter  leptosfmt
```

---

# ⛔ Anti-Patterns

> Every item here is a real bug or performance regression seen in production Leptos codebases.
> 

## Reactivity — wrong model, wrong everything

**AP-R1: Static snapshot instead of a reactive read**

```rust
// ❌ WRONG: count.get() inside view! without a closure
view! { <p>{count.get()}</p> }         // renders once, never updates

// ✅ CORRECT: signal implements IntoView (it IS a function), or wrap in closure
view! { <p>{count}</p> }               // signal is reactive
view! { <p>{move || count.get()}</p> } // explicit closure — also correct
```

**AP-R2: Writing to a signal inside an Effect to derive state**

```rust
// ❌ WRONG: two signals + one effect = unnecessary reactive nodes
let (doubled, set_doubled) = signal(0);
Effect::new(move |_| set_doubled.set(count.get() * 2));

// ✅ CORRECT: derived signal (closure) or Memo
let doubled = move || count.get() * 2;             // free, recomputes on access
let doubled = Memo::new(move |_| count.get() * 2); // cached, notifies only on change
```

**AP-R3: Using Memo for cheap computations**

```rust
// ❌ WRONG: Memo costs a reactive node + PartialEq check — not worth it for addition
let total = Memo::new(move |_| a.get() + b.get());

// ✅ CORRECT: plain closure for simple derived values
let total = move || a.get() + b.get();

// ✅ CORRECT: Memo only when computation is expensive OR you need to suppress re-renders
let filtered = Memo::new(move |_| {
    items.get().into_iter().filter(|i| i.active).collect::<Vec<_>>()
});
```

> Rule of thumb: reach for Memo when the computation takes more than a few microseconds, or when downstream nodes should not update if the value hasn't actually changed (PartialEq).
> 

**AP-R4: Writing to a signal from an Effect (potential loop)**

```rust
// ❌ DANGEROUS: can produce an infinite update loop
Effect::new(move |_| {
    let val = source.get();
    derived_signal.set(val * 2); // writing inside an effect
});

// ✅ CORRECT: Effects are strictly for external side effects
Effect::new(move |_| {
    let val = count.get();
    web_sys::console::log_1(&format!("count: {val}").into()); // side effect to the outside world
});
```

**AP-R5: `RwSignal<Vec<RwSignal<T>>>` without disposal discipline**

```rust
// ❌ WRONG: inner signals leak when outer Vec shrinks
let items: RwSignal<Vec<RwSignal<Item>>> = RwSignal::new(vec![]);
items.update(|v| v.pop()); // the inner signal is still alive in the arena

// ✅ CORRECT: use reactive_stores for structured/nested data
#[derive(Store)]
struct AppState {
    items: Vec<Item>,
}
// Per-field fine-grained reactivity, proper disposal, no leaks
```

---

## view! macro — common format mistakes

**AP-V1: `value=signal` instead of `prop:value=signal` on inputs**

```rust
// ❌ WRONG: value="..." only sets the HTML attribute at init — input stops updating
view! { <input value=name/> }

// ✅ CORRECT: prop:value= sets the live DOM property
view! {
    <input
        prop:value=name
        on:input:target=move |ev| set_name.set(ev.target().value())
    />
}
```

**AP-V2: Type mismatch between branches in view!**

```rust
// ❌ WRONG: if/else returning two different types — compile error in Leptos 0.7+
view! {
    {if show.get() { view!{<ComponentA/>} } else { view!{<ComponentB/>} }}
}

// ✅ CORRECT: use Either, or use <Show> for boolean toggles
use leptos::either::Either;
view! {
    {if show.get() {
        Either::Left(view!{<ComponentA/>})
    } else {
        Either::Right(view!{<ComponentB/>})
    }}
}
// OR, for simple boolean toggle:
view! {
    <Show when=move || show.get() fallback=|| view!{<ComponentB/>}>
        <ComponentA/>
    </Show>
}
```

**AP-V3: Keying `<For>` by index**

```rust
// ❌ WRONG: list reorders → wrong keys → potential panics or wrong UI
view! {
    <For each=move || items.get() key=|item| item.index children=|item| .../>
}

// ✅ CORRECT: always use a stable, unique ID per row
view! {
    <For each=move || items.get() key=|item| item.id children=|item| .../>
}
```

**AP-V4: Inline closure branching instead of `<Show>`**

```rust
// ❌ SUBOPTIMAL: entire closure re-runs whenever ANY signal inside it changes
view! {
    {move || if cond.get() { view!{<HeavyComponent/>} } else { view!{<Fallback/>} }}
}

// ✅ CORRECT: <Show> memoizes the predicate, only re-creates the branch when the boolean flips
view! {
    <Show when=move || cond.get() fallback=|| view!{<Fallback/>}>
        <HeavyComponent/>
    </Show>
}
```

---

## Types & ownership — Rust specifics

**AP-T1: Cloning signals instead of copying them**

```rust
// ❌ UNNECESSARY: RwSignal is Copy + 'static
let count2 = count.clone();

// ✅ CORRECT: just copy
let count2 = count;
```

**AP-T2: Lifetime parameters on components instead of owned types**

```rust
// ❌ WRONG: explicit lifetimes cause pain under Rust 2024's RPIT capture rules
#[component]
fn Label<'a>(text: &'a str) -> impl IntoView { ... }

// ✅ CORRECT: use owned types — the community has recommended this for years
#[component]
fn Label(text: String) -> impl IntoView { ... }
// or zero-copy with Oco:
#[component]
fn Label(#[prop(into)] text: Oco<'static, str>) -> impl IntoView { ... }
```

**AP-T3: `usize` / `isize` for data that crosses any boundary**

```rust
// ❌ WRONG: WASM is 32-bit; values above u32::MAX silently truncate
struct Item { id: usize }

// ✅ CORRECT: explicit integer width
struct Item { id: u64 }
```

**AP-T4: `#[cfg(feature = "ssr")]` gating an entire server function module**

```rust
// ❌ WRONG: the client stub is hidden → client-side invocation breaks at compile time
#[cfg(feature = "ssr")]
pub mod api { ... }

// ✅ CORRECT: gate only the body, keep the signature visible to both targets
#[server]
pub async fn my_fn() -> Result<Data, ServerFnError> {
    // server-only imports live INSIDE the body
    use crate::db::pool;
    ...
}
```

---

## CSR-specific pitfalls

**AP-C1: Importing `leptos_axum` or SSR-only crates in a CSR build**

```toml
# ❌ WRONG in vedge-app (CSR/WASM) — panics at runtime inside WASM
[dependencies]
leptos_axum = { version = "0.8" }

# ✅ CORRECT: use the csr feature only
leptos = { version = "0.8", features = ["csr"] }
```

**AP-C2: Using cargo-leptos for a CSR project**

```bash
# ❌ WRONG: cargo-leptos is for SSR / hydration workflows
cargo leptos watch

# ✅ CORRECT: Trunk for CSR
trunk serve
trunk build --release
```

**AP-C3: `tokio::spawn` or `spawn_blocking` inside WASM**

```rust
// ❌ WRONG: Tokio does not exist in WASM
tokio::spawn(async move { ... });

// ✅ CORRECT: use Leptos's task API or wasm-bindgen-futures
leptos::task::spawn_local(async move { ... });
```

---

# Reactive System — mental model

Leptos uses **fine-grained reactivity**: each component function runs **exactly once**. Only the closures inside `view!` that wrap reactive reads re-execute when a tracked signal changes. This is fundamentally different from React's re-render model.

```
Signal changes → only the closure tracking that signal re-runs
               → the component function itself does NOT re-run
```

## Signal type hierarchy

| Type | When to use | Notes |
| --- | --- | --- |
| `signal(T)` → `(ReadSignal, WriteSignal)` | Local component state; want to separate read / write access clearly | Default choice. Pass `ReadSignal` down to children to restrict writes. |
| `RwSignal::new(T)` | State that must be stored in a struct or passed to many places | `Copy + 'static`. Don't use everywhere — prefer split signals when possible. |
| `Memo::new(|_| expr)` | Expensive derived computation, or suppressing downstream updates when the value hasn't changed | Allocates a reactive node. Uses `PartialEq` to suppress notifications. |
| `move || expr` | Simple derived values — the default for derived state | Zero allocation; recomputes on every access. Usually sufficient. |
| `StoredValue::new(T)` | Non-reactive values: callbacks, config, NodeRef, heavy structs | `Copy + 'static`, no reactivity overhead. |
| `ArcRwSignal`, `ArcReadSignal` | List items with individual signals, cross-thread use, or signals that outlive their parent owner | Reference-counted instead of arena-allocated. |

## Owner & disposal

Signals in Leptos 0.7+ use **arena-based ownership** — not reference counting. A signal lives until its owning scope is dropped. This is why `RwSignal<Vec<RwSignal<T>>>` leaks: inner signals are not disposed when the outer Vec shrinks.

**Prefer `#[derive(Store)]`** from `reactive_stores` for structured and nested state.

---

# Component patterns

## Anatomy of a component

```rust
use leptos::prelude::*;

#[component]
pub fn UserCard(
    // Owned props — no lifetimes
    name: String,
    // Signal props — #[prop(into)] lets callers pass ReadSignal, Memo, or plain T
    #[prop(into)] active: Signal<bool>,
    // Optional
    #[prop(optional)] subtitle: Option<String>,
    // Default value
    #[prop(default = "md".to_string())] size: String,
    // Callback — prefer Callback<T> over bare Fn
    on_click: Callback<ev::MouseEvent>,
    // Children — only when needed
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class="user-card"
            class:active=active
            class=("size-md", move || size == "md")
        >
            <h3>{name}</h3>
            {subtitle.map(|s| view!{ <p>{s}</p> })}
            <button on:click=on_click>"Details"</button>
            {children()}
        </div>
    }
}
```

## Prop modifiers

| Modifier | When to use |
| --- | --- |
| `#[prop(into)]` | Lets the caller pass `ReadSignal<T>`, `Memo<T>`, `Signal<T>`, or plain `T` — all converted automatically |
| `#[prop(optional)]` | Makes the prop `Option<T>`, defaulting to `None` |
| `#[prop(default = expr)]` | Provides a concrete default; caller can omit the prop |
| `#[prop(strip_option)]` | Caller passes `T`, component receives `Option<T>` |

## Children types

| Type | When to use |
| --- | --- |
| `Children` | Default — render once |
| `ChildrenFn` | Render multiple times (e.g. inside `<Show>` fallback, custom `<For>`) |
| `TypedChildren<T>` | Preserve the concrete type of children — avoids type erasure overhead |
| `ChildrenFragment` | Need to iterate over individual children |

---

# view! macro — best practices

## Dynamic attributes

```rust
view! {
    <button
        // Toggle a class reactively
        class:active=move || is_active.get()
        // Class name with special characters
        class=("btn-primary", move || variant.get() == "primary")
        // Reactive inline style
        style:opacity=move || if loading.get() { "0.5" } else { "1" }
        // Reactive attribute
        aria-pressed=move || is_pressed.get().then_some("true")
        // DOM property — not HTML attribute
        prop:value=move || current_value.get()
        // Typed event with :target shorthand
        on:input:target=move |ev| set_value.set(ev.target().value())
    >
        "Submit"
    </button>
}
```

## Control flow

```rust
// Boolean toggle — use <Show>
view! {
    <Show when=move || is_logged_in.get() fallback=|| view!{<Login/>}>
        <Dashboard/>
    </Show>
}

// Dynamic list — always use <For> with a stable key
view! {
    <For
        each=move || items.get()
        key=|item| item.id       // stable unique ID — never index
        children=|item| view!{ <ItemRow item=item/> }
    />
}

// Multi-branch — use Either or EitherOf3/4
use leptos::either::EitherOf3;
view! {
    {move || match status.get() {
        Status::Loading => EitherOf3::A(view!{<Spinner/>}),
        Status::Error   => EitherOf3::B(view!{<ErrorView/>}),
        Status::Done    => EitherOf3::C(view!{<Content/>}),
    }}
}
```

---

# State management

## Local vs global state

```
Local (inside a component)   → signal(T) / RwSignal::new(T)
Shared (parent → children)   → pass signal as prop
Global (app-wide)            → provide_context / #[derive(Store)]
```

## Context API

```rust
// Provider — typically at App root
#[derive(Clone)]
struct AuthState {
    user: RwSignal<Option<User>>,
    is_authenticated: Memo<bool>,
}

#[component]
fn App() -> impl IntoView {
    let user = RwSignal::new(None::<User>);
    let auth = AuthState {
        user,
        is_authenticated: Memo::new(move |_| user.get().is_some()),
    };
    provide_context(auth);
    view! { <Router/> }
}

// Consumer — any descendant
#[component]
fn Navbar() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    view! {
        <Show when=move || auth.is_authenticated.get()>
            <UserMenu/>
        </Show>
    }
}
```

> **Important:** Context is keyed by type. Wrap state in a newtype to prevent silent collisions: `struct DarkMode(RwSignal<bool>);`
> 

## Reactive Stores for structured state

```rust
use reactive_stores::Store;

#[derive(Store, Clone)]
pub struct VaultState {
    pub entries: Vec<Entry>,
    pub is_locked: bool,
    pub active_entry_id: Option<u64>,
}

// Fine-grained reactivity per field.
// Writing to `entries` does NOT trigger re-renders subscribed to `is_locked`.
#[component]
fn VaultView() -> impl IntoView {
    let store = Store::new(VaultState::default());
    provide_context(store);

    let is_locked = store.is_locked();
    view! {
        <Show when=move || !is_locked.get()>
            <EntryList/>
        </Show>
    }
}
```

---

# Async — Resources & Actions (CSR)

## Resource — data fetching

```rust
// Resource with a source signal (refetches when source changes)
let user_id = RwSignal::new(1u64);
let user = Resource::new(
    move || user_id.get(),                      // source: tracked, reactive
    |id| async move { fetch_user(id).await },   // fetcher: untracked async
);

// LocalResource — CSR-only, for !Send browser APIs
let data = LocalResource::new(|| async {
    fetch_from_tauri_command().await
});

// Reading in a view — must be inside <Suspense>
view! {
    <Suspense fallback=|| view!{<Spinner/>}>
        {move || Suspend::new(async move {
            match user.await {
                Ok(u)  => view!{<UserCard user=u/>}.into_any(),
                Err(e) => view!{<ErrorMsg msg=e.to_string()/>}.into_any(),
            }
        })}
    </Suspense>
}
```

## Action — mutations

```rust
// Action for side effects: save, delete, Tauri commands
let save_entry = Action::new(|entry: &Entry| {
    let entry = entry.clone();
    async move { save_to_store(entry).await }
});

view! {
    <button
        disabled=move || save_entry.pending().get()
        on:click=move |_| { save_entry.dispatch(current_entry.get()); }
    >
        {move || if save_entry.pending().get() { "Saving..." } else { "Save" }}
    </button>
}

// Action exposes reactive state:
// .pending()  → Signal<bool>
// .value()    → Signal<Option<Result<T, E>>>
// .input()    → Signal<Option<I>>
```

## Suspense vs Transition

|  | `<Suspense>` | `<Transition>` |
| --- | --- | --- |
| When resource already has a value and refetches | Falls back to the loading UI | Keeps showing stale content; optional `set_pending` signal |
| Use when | Loading is a distinct, intentional UI state (first load) | Tab switch, filter change — avoid flicker |

---

# Rust 2024 Edition — what actually changes

Rust 2024 is stable since Rust 1.85 (Feb 2025). The vast majority of Leptos code is **unaffected**. Know these:

## RPIT lifetime capture (RFC 3498)

Before (2021): `-> impl Trait` did not capture lifetime parameters.

After (2024): all in-scope params are captured automatically — can trigger `E0521` if a component uses explicit lifetimes.

```rust
// If you genuinely need to drop a lifetime in Rust 2024:
fn my_fn<'a>(data: &'a Data) -> impl IntoView + use<> { ... }
//                                               ^^^^^ don't capture 'a
```

**Simplest fix:** use owned types (`String` instead of `&str`). The Leptos community has recommended this for years — Rust 2024 makes it mandatory in edge cases.

## Async closures — pure upside

```rust
// Rust ≥ 1.85: native async closures, no more workarounds
let action = Action::new(async |entry: &Entry| {
    save(entry.clone()).await
    // Previously required: let e = entry.clone(); async move { save(e).await }
});
```

## `gen` is now a reserved keyword

```rust
// If using an older version of rand:
rng.r#gen::<u64>() // escape with r#
// Or update rand to 0.9+ which renamed the method
```

## Migration workflow

```bash
# 1. Ensure clean build on edition 2021 first
cargo check --all-features

# 2. Run the automated migration
cargo fix --edition --all-features

# 3. Bump Cargo.toml
# edition = "2024"
# rust-version = "1.85"

# 4. Review residual warnings
cargo clippy --all-targets --all-features
```

Add a safety net before migrating:

```toml
# lib.rs or Cargo.toml [lints.rust]
#![warn(rust_2024_compatibility)]
```

---

# Project structure (CSR — vedge-app)

```
crates/vedge-app/
└── src/
    ├── main.rs              // Leptos mount: leptos::mount::mount_to_body(App)
    ├── app.rs               // <App/> root, router, global context providers
    ├── components/          // Reusable UI — uses vedge-ui types; unaware of vault logic
    │   ├── mod.rs
    │   ├── vault_entry.rs
    │   └── pki_badge.rs
    ├── pages/               // Route-level views
    │   ├── mod.rs
    │   ├── vault.rs
    │   ├── settings.rs
    │   └── pki.rs
    ├── state/               // Global reactive state (Store + Context)
    │   ├── mod.rs
    │   ├── vault_state.rs   // #[derive(Store)]
    │   └── auth_state.rs
    └── bridge/              // Tauri invoke wrappers
        ├── mod.rs
        └── vault_bridge.rs  // async fn → tauri::invoke("command_name", ...)
```

## Tauri bridge pattern

In vedge, "server functions" are Tauri commands. Standard pattern:

```rust
// bridge/vault_bridge.rs
pub async fn unlock_vault(password: String) -> Result<SessionInfo, BridgeError> {
    let args = serde_wasm_bindgen::to_value(
        &serde_json::json!({ "password": password })
    )?;
    let result = tauri_sys::tauri::invoke("unlock_vault", &args)
        .await
        .map_err(|e| BridgeError::Invoke(e.to_string()))?;
    serde_wasm_bindgen::from_value(result).map_err(Into::into)
}

// Used in a component via LocalResource or Action
let unlock = Action::new(|pwd: &String| {
    let pwd = pwd.clone();
    async move { unlock_vault(pwd).await }
});
```

---

# Tooling

## Trunk config (CSR)

```toml
# Trunk.toml
[build]
target = "index.html"

[watch]
ignore = ["./target"]

[serve]
address = "127.0.0.1"
port    = 8080
open    = false

[[hooks]]
stage             = "post_build"
command           = "sh"
command_arguments = ["-c", "wasm-opt -Oz --enable-bulk-memory -o dist/app_bg.wasm dist/app_bg.wasm"]
```

## Cargo.toml essentials

```toml
[dependencies]
leptos              = { version = "0.8", features = ["csr"] }
reactive_stores     = "0.2"
wasm-bindgen        = "0.2"
wasm-bindgen-futures = "0.4"
serde               = { version = "1", features = ["derive"] }
thiserror           = "2"

[profile.wasm-release]
inherits      = "release"
opt-level     = 'z'
lto           = true
codegen-units = 1
panic         = "abort"
strip         = true
```

## leptosfmt

```bash
cargo install leptosfmt
```

```toml
# leptosfmt.toml
max_width        = 100
tab_spaces       = 4
closing_tag_style = "Preserve"
```

Wire into rust-analyzer:

```json
{
  "rust-analyzer.rustfmt.overrideCommand": ["leptosfmt", "--stdin", "--rustfmt"]
}
```

## Clippy config

```toml
# Cargo.toml
[lints.clippy]
needless_lifetimes = "allow"  # #[component] macro breaks lifetime elision
too_many_arguments = "allow"  # components with many props
empty_docs         = "allow"  # proc-macro expansion artifact
```

## rust-analyzer settings

```json
{
  "rust-analyzer.cargo.features": ["csr"]
}
```

---

# Testing

## Tier 1 — Pure logic

```rust
// Any business logic without signals → plain unit tests
#[test]
fn test_entry_validation() {
    assert!(Entry::validate_password("hunter2").is_err());
}
```

## Tier 2 — Reactive tests

```rust
use leptos::prelude::*;
use any_spawner::Executor;

#[tokio::test]
async fn signal_derives_correctly() {
    let _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let (count, set_count) = signal(0i32);
    let doubled = Memo::new(move |_| count.get() * 2);

    set_count.set(5);
    Executor::tick().await;
    assert_eq!(doubled.get_untracked(), 10);
}
```

## Tier 3 — WASM / DOM tests

```rust
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn component_renders() {
    let document = web_sys::window().unwrap().document().unwrap();
    let el = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&el).unwrap();

    leptos::mount::mount_to(el.clone().unchecked_into(), || view!{<MyComponent/>});
    leptos::task::tick().await;

    assert!(el.inner_html().contains("expected text"));
}
```

Run with:

```bash
wasm-pack test --chrome --headless
```

---

# Implementation checklist

- [ ]  `leptos = { version = "0.8", features = ["csr"] }` — not `ssr` or `hydrate`
- [ ]  `edition = "2024"`, `rust-version = "1.85"` in Cargo.toml
- [ ]  `leptosfmt` installed and wired into rust-analyzer
- [ ]  `[lints.clippy]` configured in Cargo.toml
- [ ]  Every signal read inside `view!` is reactive (closure or signal as function)
- [ ]  `<For>` uses a stable unique ID as key — never index
- [ ]  Controlled inputs use `prop:value=signal` + `on:input` handler
- [ ]  No `RwSignal<Vec<RwSignal<T>>>` — use `#[derive(Store)]` instead
- [ ]  Structured state uses `reactive_stores::Store`
- [ ]  No `tokio::spawn` in WASM — use `leptos::task::spawn_local`
- [ ]  Tauri commands wrapped in `bridge/` module — not called directly from components
- [ ]  Business logic extracted into pure functions → testable with `#[test]`
- [ ]  `wasm-opt` configured in Trunk.toml or `wasm-release` profile