//! `vedge-app` — the CSR/WASM Leptos frontend (bin crate).
//!
//! Adopts the workspace lint gate (`[lints] workspace = true`). The crate-level
//! `#![allow(...)]`s below are documented exceptions for a presentation/CSR-WASM app —
//! noise here, not silenced bugs. Crash-risk restriction lints (unwrap/expect/panic/
//! indexing) are NOT allowed crate-wide; they're hand-fixed. The one exception is the
//! macro-generated `i18n.rs` (included below), whose `expect`s can't be annotated in
//! place — it's wrapped in an allow-scoped module.
#![allow(
    // Non-adversarial presentation math (bounded layout/time/geometry values).
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    // CSR/WASM is single-threaded — `!Send` futures are fine.
    clippy::future_not_send,
    // Fire-and-forget `web_sys` / async-IPC `Result`s in event handlers.
    clippy::let_underscore_must_use,
    // Pedantic/nursery style that is noise for a UI app crate.
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::ignored_unit_patterns,
    clippy::used_underscore_binding,
    clippy::default_trait_access,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::redundant_closure,
    clippy::trivially_copy_pass_by_ref,
    clippy::wrong_self_convention,
    clippy::unused_self,
    clippy::struct_field_names,
    clippy::module_inception,
    clippy::many_single_char_names,
    clippy::return_self_not_must_use,
    clippy::literal_string_with_formatting_args,
    clippy::items_after_statements,
    clippy::ref_option,
    // Perf/style nursery whose `--fix` is unreliable inside `view!` macros; residual
    // after the autofix pass. Pure style — `Self`-repetition and `clone_from`.
    clippy::use_self,
    clippy::assigning_clones,
)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::unwrap_in_result,
    )
)]

mod api;
mod app;
mod features;
mod pages;
mod routes;

// The i18n module is macro-generated (`leptos_i18n::load_locales!` → a single-line
// `pub mod i18n { … }`), so its `expect`/`unwrap` can't be annotated in place. Wrap
// the `include!` in an allow-scoped module and re-export `i18n` to the crate root so
// `crate::i18n::*` still resolves.
#[allow(
    dead_code,
    unused_imports,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    // `build.rs` regenerates `i18n.rs` on every build, so the generated
    // `Locale::direction` match (identical `LeftToRight` arms for `en`/`vi`)
    // can't be hand-merged in place — allow it here, alongside the other
    // generated-code lints this wrapper already scopes.
    clippy::match_same_arms
)]
mod i18n_generated {
    include!("i18n.rs");
}
use i18n_generated::i18n;

use app::App;
use leptos::prelude::*;

fn main() {
    leptos::mount::mount_to_body(|| view! { <App /> });
}
