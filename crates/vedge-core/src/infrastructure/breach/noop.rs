//! A breach checker that never reports a breach.

use async_trait::async_trait;

use crate::application::vault::ports::breach::BreachChecker;
use crate::domain::vault::errors::VaultError;

/// Always returns an empty range — no secret is ever flagged. Useful where the
/// port is required structurally but no lookup should occur.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopBreachChecker;

impl NoopBreachChecker {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BreachChecker for NoopBreachChecker {
    async fn range(&self, _prefix: &str) -> Result<Vec<(String, u32)>, VaultError> {
        Ok(Vec::new())
    }
}
