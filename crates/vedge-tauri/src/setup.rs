//! Composition root for the Tauri shell. Everything app-lifetime lives
//! under this tree.

pub mod services;

pub use services::{ComposeError, compose};
