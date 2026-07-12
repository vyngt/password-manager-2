//! In-memory breach double. Never touches the network.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::vault::ports::breach::BreachChecker;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::health::{HIBP_PREFIX_LEN, sha1_upper_hex};

/// An offline breach double built from known-breached plaintext passwords.
///
/// Each password is hashed exactly like the real checker, so `range` answers by
/// k-anonymity prefix just as HIBP would. It also **records the prefixes it was
/// asked**, so a test can prove that only the 5-hex prefix ever crosses (never a
/// password, a full hash, or a suffix). Also serves the e2e offline seam.
pub struct MemoryBreachChecker {
    /// prefix (5 upper-hex) -> [(suffix (35 upper-hex), count)]
    corpus: HashMap<String, Vec<(String, u32)>>,
    asked: Mutex<Vec<String>>,
}

impl MemoryBreachChecker {
    /// An empty corpus — every lookup returns no match.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            corpus: HashMap::new(),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// Build a corpus from `(password, count)` pairs. Each password is SHA-1'd
    /// and indexed by its 5-hex prefix, so `range(prefix)` returns the matching
    /// suffixes — the same shape the HIBP range API returns.
    #[must_use]
    pub fn with_breached(entries: &[(&str, u32)]) -> Self {
        let mut corpus: HashMap<String, Vec<(String, u32)>> = HashMap::new();
        for (password, count) in entries {
            let hex = sha1_upper_hex(password);
            let (prefix, suffix) = hex.split_at(HIBP_PREFIX_LEN);
            corpus
                .entry(prefix.to_owned())
                .or_default()
                .push((suffix.to_owned(), *count));
        }
        Self {
            corpus,
            asked: Mutex::new(Vec::new()),
        }
    }

    /// Test hook: the prefixes `range` was asked for (uppercase, 5 hex each).
    /// Not part of the port.
    #[must_use]
    pub fn asked_prefixes(&self) -> Vec<String> {
        self.asked.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl Default for MemoryBreachChecker {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait]
impl BreachChecker for MemoryBreachChecker {
    async fn range(&self, prefix: &str) -> Result<Vec<(String, u32)>, VaultError> {
        if let Ok(mut asked) = self.asked.lock() {
            asked.push(prefix.to_owned());
        }
        Ok(self.corpus.get(prefix).cloned().unwrap_or_default())
    }
}
