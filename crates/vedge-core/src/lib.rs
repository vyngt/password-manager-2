#![cfg_attr(test, allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    clippy::needless_pass_by_value,
    dead_code,
))]

pub mod application;
pub mod domain;
pub mod infrastructure;
