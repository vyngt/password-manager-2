//! `vedge-ui` — the CSR/WASM design-system crate.
//!
//! Adopts the workspace lint gate (`[lints] workspace = true`). The crate-level
//! `#![allow(...)]`s below are documented exceptions appropriate to a *presentation*
//! layer — each is noise here, not a silenced bug. The crash-risk restriction lints
//! (`unwrap`/`expect`/`panic`/`indexing`/`unreachable`) are deliberately NOT allowed:
//! they're hand-fixed, because a panic crashes the app. Mirrors the module-scoped
//! precedent at `vedge-tauri/src/pdf/emergency_kit.rs`.
#![allow(
    // Non-adversarial presentation math — bounded color/geometry/pixel values, never
    // attacker-controlled. The overflow/precision restriction lints guard vedge-core's
    // crypto/data paths, not a renderer.
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::suboptimal_flops,
    // Intentional exact float compares — max-channel selection in RGB→HSV, the
    // `val == val.trunc()` integer-valued check, HSL black/white (0.0/1.0) edge guards.
    // Verified not bugs.
    clippy::float_cmp,
    clippy::neg_cmp_op_on_partial_ord,
    // Fire-and-forget `web_sys` DOM calls whose `Result` is correct to ignore.
    clippy::let_underscore_must_use,
    // Pedantic/nursery style that is noise for a component crate (long render fns,
    // getter/self conventions, math single-char names) or whose autofix is unreliable
    // inside `view!` macros.
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::redundant_closure,
    clippy::trivially_copy_pass_by_ref,
    clippy::literal_string_with_formatting_args,
    clippy::wrong_self_convention,
    clippy::unused_self,
    clippy::fn_params_excessive_bools,
    clippy::struct_field_names,
    clippy::module_inception,
    clippy::future_not_send,
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

pub mod components;
pub mod primitives;
pub mod theme;
pub mod utils;
