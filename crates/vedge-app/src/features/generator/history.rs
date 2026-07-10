//! Session-only history of panel-generated secrets (slice 3.5).
//!
//! A small, capped, in-memory log of secrets the generator panel produced, so
//! the user can re-copy or re-use a recent one. This is the app's **first
//! explicit session-secret wipe** (see the memory-hygiene pre-flight):
//!
//! - every entry is a [`Zeroizing<String>`] — evicted/cleared items zeroize on drop;
//! - the buffer is capped at [`HISTORY_CAP`], bounding the number of live copies;
//! - [`HistoryItem`] is deliberately **not** `Serialize`/`Debug`, so it can neither
//!   be written to `app.db`/the vault nor formatted into a log;
//! - it lives in an app-root [`GeneratedHistoryCtx`] and is emptied whenever the
//!   vault locks (a prev-guarded `Effect` in `app.rs`).
//!
//! History is a *deliberate* set of copies — the "secrets move, never clone" rule
//! can't apply to a review buffer — so we bound it instead (cap + `Zeroizing` +
//! wipe-on-lock + non-serializable + a Clear button).

use leptos::prelude::*;
use zeroize::Zeroizing;

/// A single generated secret retained for the session.
///
/// Intentionally **not** `Debug`/`Serialize`: it must never be logged or
/// persisted. `Clone` only, so the reactive `<For>` list can key over it.
#[derive(Clone)]
pub struct HistoryItem {
    /// Stable, monotonically-assigned id for `<For>` keying (never a secret).
    pub id: u64,
    /// The generated characters — wiped on drop.
    pub secret: Zeroizing<String>,
    /// Entropy of the process that produced it (drives the per-row band).
    pub entropy_bits: f64,
}

/// Maximum number of secrets retained in the session history.
pub const HISTORY_CAP: usize = 20;

/// Prepend `item` (newest-first) and truncate the buffer to `cap`.
///
/// Evicted tail entries drop here → their `Zeroizing` secrets are wiped. Kept
/// Leptos-free so it's host-testable; the caller stamps `item.id`.
pub fn push_history(buf: &mut Vec<HistoryItem>, item: HistoryItem, cap: usize) {
    buf.insert(0, item);
    buf.truncate(cap);
}

/// App-root reactive context holding the session history buffer plus a monotonic
/// id source. `RwSignal<T>` is `Copy` regardless of `T`, so the wrapper is `Copy`
/// (mirrors `GeneratorPrefsCtx`).
#[derive(Clone, Copy)]
pub struct GeneratedHistoryCtx {
    /// Newest-first, capped at [`HISTORY_CAP`].
    pub items: RwSignal<Vec<HistoryItem>>,
    /// Ever-increasing id source; stamped onto each pushed item so `<For>` keys
    /// stay stable and collision-free across both panel mounts.
    pub next_id: RwSignal<u64>,
}

impl GeneratedHistoryCtx {
    /// Construct an empty history context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(0),
        }
    }

    /// Stamp a fresh id, prepend the secret, and truncate to [`HISTORY_CAP`].
    pub fn push(&self, secret: Zeroizing<String>, entropy_bits: f64) {
        let id = self.next_id.get_untracked();
        self.next_id.set(id.wrapping_add(1));
        self.items.update(|buf| {
            push_history(
                buf,
                HistoryItem {
                    id,
                    secret,
                    entropy_bits,
                },
                HISTORY_CAP,
            );
        });
    }

    /// Empty the buffer (Clear button and wipe-on-lock); dropped items zeroize.
    pub fn clear(&self) {
        self.items.set(Vec::new());
    }
}

impl Default for GeneratedHistoryCtx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, secret: &str) -> HistoryItem {
        HistoryItem {
            id,
            secret: Zeroizing::new(secret.to_owned()),
            entropy_bits: 42.0,
        }
    }

    #[test]
    fn push_history_prepends_newest_first() {
        let mut buf = Vec::new();
        push_history(&mut buf, item(0, "aaa"), HISTORY_CAP);
        push_history(&mut buf, item(1, "bbb"), HISTORY_CAP);
        push_history(&mut buf, item(2, "ccc"), HISTORY_CAP);
        let ids: Vec<u64> = buf.iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![2, 1, 0]);
        assert_eq!(buf[0].secret.as_str(), "ccc");
    }

    #[test]
    fn push_history_truncates_to_cap() {
        let mut buf = Vec::new();
        for id in 0..25 {
            push_history(&mut buf, item(id, "x"), HISTORY_CAP);
        }
        assert_eq!(buf.len(), HISTORY_CAP);
        // Newest retained is id 24; oldest retained is id 5 (0..=4 evicted).
        assert_eq!(buf.first().map(|i| i.id), Some(24));
        assert_eq!(buf.last().map(|i| i.id), Some(5));
    }

    #[test]
    fn push_history_respects_small_cap() {
        let mut buf = Vec::new();
        for id in 0..5 {
            push_history(&mut buf, item(id, "x"), 3);
        }
        let ids: Vec<u64> = buf.iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![4, 3, 2]);
    }
}
