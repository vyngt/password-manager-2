# Dialog ✕ Close Bug — Root-Cause Analysis (SOLVED)

*2026-07-09 · branch `feature/2.9.1-layout-forms-scrollbars` (PR #24)*

## Executive summary

**Root cause: `DialogContext` leaked across sibling `Dialog` instances.** `Dialog` provided its
context (`on_close`, `close_label`, title/body id setters) via `provide_context` in its component
body; with several Dialogs mounted on one page, **every `DialogHeader`'s `expect_context` resolved
the LAST-mounted Dialog's context** — not its own. The ✕ therefore faithfully closed a *different,
already-closed* dialog: its signal was written (`false` → `false`, no change), its memo recomputed,
and nothing visible happened, while the on-screen dialog never received a close at all.

**Fix:** provide the context through an explicit **`<leptos::context::Provider value=…>`** scope
wrapping the dialog subtree (portal > provider > `<Show>` > scrim), so each dialog's sub-components
unambiguously resolve *their own* context. Verified with an automated headless-browser rig:
✕ / backdrop / Escape / footer all close, 8/8 rapid open→✕ cycles tear down perfectly, and the
console is warning-free.

This was **not** a version regression and **not** a Portal/Show/teardown bug. Two earlier committed
"fixes" targeted the wrong layer and are retained only where they independently improved the code.

## 1. Why this bug resisted five fixes

The bug produced *plausible evidence for every wrong theory*:

- The ✕ handler **did fire** and **did flip a signal** — console traces showed
  `on_close` running and `open=false` — so callback wiring looked correct. The trap: it was the
  **wrong instance's** handler evidence being read as the right instance's. Untagged logs
  (`[Dialog] Show when -> false` with no instance id) made the wrong dialog's recompute
  indistinguishable from the right one's.
- Backdrop/Escape/footer **worked**, so the delta appeared to be "spread events", "Portal
  boundaries", "deferral timing" — anything *positional*. The real delta: those paths capture
  `on_close` **lexically** (Dialog's own view / the consumer page's view); only the ✕ path resolves
  it **via context**.
- The "`mounted` desync" and "one-behind catch-up" observations were both artifacts of writes
  landing on the wrong instance: `set(true)` on an already-open signal and `set(false)` on an
  already-closed one produce recomputes with no value change — silence that mimicked a reactive
  teardown bug.
- A **stale-build window** (Trunk did not watch `vedge-ui/src`, fixed in `2c0b6b0`; the `[watch]`
  section still isn't in `Trunk.toml` — see §5) invalidated several manual test rounds, including
  one that had *correctly* exonerated a suspect.

## 2. The definitive experiment

An automated rig (headless Edge driven over CDP against `trunk serve`, with a `window.__TAURI__`
shim) made testing reproducible and eliminated stale-build/manual-click uncertainty. The
instance-tagged trace settled it in one run:

```
visible dialog:   [Dialog#4] Show when -> true          ← the closeable demo opened
✕ clicked:        [DialogHeader] running on_close of Dialog#11   ← WRONG INSTANCE (last-mounted)
                  [Dialog#11] Show when -> false        ← closes the already-closed dialog
                  (the page's close callback never runs; #4 stays on screen)
backdrop clicked: [pg] close_cb invoked → [Dialog#4] … → cleanup → scrim removed ✓
```

Supporting grid (all on one build): a **plain `<button>` in the same header running the same
callback captured lexically** closed the dialog fine from every position (header content, body,
footer) — only the context-resolved ✕ failed. That exonerated `IconButton`, the `on_click` prop,
`<Icon>`, the `.then()` block, focus state, event trust, sync-vs-deferred writes, `<Show>`/ArcMemo,
and the `<Portal>` — each of which had been individually falsified by A/B builds before the tagged
trace pinned the context leak.

## 3. The fix (in `crates/vedge-ui/src/components/feedback/dialog/dialog.rs`)

```rust
// Context provided via an explicit scope, NOT provide_context in the body:
let dialog_ctx = DialogContext { on_close, closeable, close_label, set_title_id, set_body_id };

view! {
    <leptos::portal::Portal>
        <leptos::context::Provider value=dialog_ctx>
            <Show when=move || open.get()>
                // scrim + dialog + children (DialogHeader/Title/Body/Footer
                // resolve THIS dialog's context)
            </Show>
        </leptos::context::Provider>
    </leptos::portal::Portal>
}
```

Also shipped in the same change, each independently verified by the rig:

- **Portal permanently mounted, `<Show>` inside it.** Closing swaps a plain DOM child inside the
  portal container instead of tearing the Portal down. (One empty portal `<div>` per mounted Dialog
  remains in `<body>` — inert.)
- **Close runs synchronously** from all triggers (✕ / backdrop / Escape) — the earlier
  `set_timeout` deferral (`6f72ad4`) was removed; it was a fix aimed at the wrong layer.
- The `DialogTitle`/`DialogBody` id setters now also reach the correct instance — the same leak had
  been silently mis-wiring `aria-labelledby`/`aria-describedby` across dialogs.

## 4. Post-mortem of the fix history (kept/retracted)

| Commit | Claimed cause | Verdict | Disposition |
|---|---|---|---|
| `41d4f2e` gate `<Show>` on `open` | internal `mounted` desync | Wrong cause (context leak), but the simplification is correct | **Keep** |
| `f0cfa94` `IconButton on_click` prop | spread `on:click` doesn't attach across Portal | **Retracted** — the handler always fired | Keep the prop (good API); header uses it |
| `6f72ad4` deferred close | synchronous Portal teardown re-entrancy | **Retracted** — deferral changed nothing | **Reverted** (sync close) |
| `docs` v1 of this file | tachys 0.2.16–0.2.18 owner-disposal regression | **Falsified** — the lock has been leptos 0.8.19 + tachys 0.2.15 since April 18, unchanged through the bug window; the bug reproduces on it | Corrected here |

The uncommitted working-tree `cargo update` (leptos 0.8.20 / tachys 0.2.18) was the user's untested
attempt at a fix, made *after* the bug appeared; it was restored to the committed lock during the
investigation and stays restored — the committed versions are the long-proven pair. (The tachys
0.2.16+ "defer owner drop in OwnedView" change documented in v1 of this file is real and worth
re-testing overlays against whenever the lock is deliberately updated — it just wasn't *this* bug.)

## 5. Open questions / follow-ups

1. **Upstream:** `provide_context` from a `#[component]` body being visible to *sibling* component
   subtrees (last-provider-wins) looks like a leptos owner-scoping bug or at minimum a severe
   footgun — the docs' "nearest provider" model does not predict it. Worth a minimal repro
   (two dialogs, context-resolved close button) and an issue against leptos 0.8.19. Until the
   semantics are clarified, **prefer `<Provider>` scopes for any per-instance context in
   `vedge-ui`** (audit: Toast provides at app level — singleton, unaffected; check any future
   per-instance contexts).
2. **Trunk watch:** `[watch] paths = ["src", "index.html", "../vedge-ui/src"]` is committed in
   `Trunk.toml` (`2c0b6b0`), but a `trunk serve --config …` launched from the repo root did NOT
   pick up `vedge-ui` edits during this investigation — trunk appears to resolve the watch paths
   against the CWD, not the config file. Launch trunk from `crates/vedge-app/` (as `cargo tauri
   dev` does) or verify rebuilds by watching the served bundle hash change.
3. **Exit animation:** the birth implementation's 150 ms exit fade remains dropped (visibility is
   `<Show when=open>`). If wanted later, it can be rebuilt safely on this now-correct base.
4. **Rig:** the CDP scripts (matrix / stress / grid) live at
   `%LOCALAPPDATA%\Temp\claude\vedge-dialog-rig\` — worth adopting into `scripts/` as a headless UI
   smoke if similar issues recur.

## 6. Verification record (final build, production code, no diagnostics)

- Rig matrix: ✕ → scrims 0 ✓, backdrop → 0 ✓, Escape → 0 ✓; footer/positional grid all ✓.
- Stress: 8/8 rapid open→✕ cycles, scrims 1→0 every cycle ✓.
- Console: zero framework warnings, zero leftover diagnostics ✓.
- Gates: `cargo build -p vedge-ui` and `-p vedge-app --target wasm32-unknown-unknown` warning-free;
  `clippy --no-deps -D warnings` clean; `cargo test --workspace --lib --bins --tests` 0 failed;
  rustfmt clean on touched files.
- Remaining human smoke: the Tauri app itself (Manage-tags ✕, edit-entry dialog ✕, history dialog ✕)
  — expected to pass identically (same WebView engine family as the rig's headless Edge).

## 7. Why the leak happens — the context model, explained

Leptos context is **not keyed by component**. It lives in the reactive **owner tree**:

- `provide_context(v)` means: *insert `v` (keyed by its Rust type) into the context map of
  whatever `Owner` is current at this moment.*
- `expect_context::<T>()` means: *starting from the `Owner` current at the call site, walk up
  the ancestor chain and return the first `T` found.*

The intuitive "nearest provider wins" model silently depends on two invariants:

1. **Ancestry** — the owner that received the provide is an *ancestor* of every owner in which the
   component's descendants later call `expect_context` (so they can find it at all).
2. **Isolation** — that owner is *not shared* with sibling instances (so sibling provides go into
   *different* maps instead of overwriting one another).

Older Leptos (0.6) passed an explicit `Scope` through every component, which made these boundaries
visible and per-instance. In 0.7/0.8 owners are implicit and are created by **reactive nodes** —
effects, memos, control-flow branches, `Suspense`, `<Provider>` — *not* reliably by every
`#[component]` function call. A component body is "just a function" that runs under whatever owner
is current while the parent's view is being built.

In our composition — a page statically mounting 12 `<Dialog>`s, each Dialog rendering type-erased
`ChildrenFn` children inside a `<Portal>` (which re-roots rendering via `mount_to` inside an
`Effect`) and a `<Show>` — invariant **2** broke: the owner receiving each Dialog's body
`provide_context` was effectively shared, so twelve provides of the *same type*
(`DialogContext`) overwrote one another — **last write wins** — and every `DialogHeader`'s lookup
resolved Dialog#11's map. (Which exact hop of the owner chain misroutes — the eager component-body
execution, the erased-children owner, or the portal's `mount_to` root — is the open upstream
question in §5.1; the misrouting itself is proven by the instance-tagged trace in §2.)

`<Provider value=…>` fixes this **by construction**, not by luck: it is a *view-tree element* that
creates a dedicated child owner exactly around the wrapped subtree and provides the value there.
Both invariants become structural: the provider owner is an ancestor of precisely (and only) the
wrapped children, so each dialog's descendants must pass through *their own* provider first, and
sibling providers live in disjoint owners. There is nothing left to depend on about how component
bodies map to owners.

## 8. Guideline — context in `vedge-ui` components

**Rule: any context that is *per-instance* — provided by a component that can be mounted more than
once per page — MUST be provided with an explicit `<Provider value=…>` wrapping exactly the subtree
that consumes it. Never `provide_context()` from the `#[component]` body for such types.**

- **Litmus test:** *"Could two live instances of this component exist in one page?"* Dialog,
  any compound Header/Body/Footer pattern, cards, rows, popovers → yes → `<Provider>`.
  It doesn't matter that "only one dialog is open at a time" — **mounted** is what counts, and
  every `<Dialog>` in a view is mounted even while closed.
- **App-level singletons** (ToastProvider, ThemeState, i18n, `VaultUiState`) may keep body
  `provide_context`: with exactly one provider of the type there is no last-write-wins hazard.
  Provide them once, near the root, unconditionally.
- **Portals:** place the `<Provider>` *inside* the portal content
  (`Portal > Provider > …`), so the `mount_to` re-rooting cannot bypass it. This is the shipped
  Dialog structure.
- **Consumers:** a sub-component that `expect_context`s a per-instance type is only valid inside
  its provider's subtree — say so in its doc comment (a panic means "mounted outside X").
- **Diagnosing context-identity bugs:** tag logs with a per-instance id (`static AtomicU32`)
  *before* trusting any trace — untagged logs from N instances are indistinguishable, and that
  ambiguity cost this bug five wrong fixes. The headless-CDP rig (§5.4) turns each hypothesis
  into a minutes-long A/B test.
- **Review checklist addition:** any new `provide_context` in a PR → ask "multiple live instances
  possible?" If yes, require the `<Provider>` form.
