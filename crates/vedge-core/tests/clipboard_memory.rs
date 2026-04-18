#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use vedge_core::application::vault::ports::ClipboardProvider;
use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;

#[test]
fn set_then_peek_returns_value() {
    let cb = MemoryClipboardProvider::new();
    cb.set("hunter2").unwrap();
    assert_eq!(cb.peek().as_deref(), Some("hunter2"));
}

#[test]
fn clear_empties() {
    let cb = MemoryClipboardProvider::new();
    cb.set("hunter2").unwrap();
    cb.clear().unwrap();
    assert_eq!(cb.peek(), None);
}

#[test]
fn set_overwrites_previous() {
    let cb = MemoryClipboardProvider::new();
    cb.set("first").unwrap();
    cb.set("second").unwrap();
    assert_eq!(cb.peek().as_deref(), Some("second"));
}
