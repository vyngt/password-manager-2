use crate::domain::vault::errors::VaultError;

/// Synchronous wrapper around the OS clipboard.
///
/// Deliberately runtime-agnostic — the 30-second clear timer used by
/// `CopyField` spawns its own async task with an `Arc<dyn ClipboardProvider>`
/// so the port itself has no `tokio` dependency.
pub trait ClipboardProvider: Send + Sync {
    /// Place `text` on the clipboard, replacing whatever is there.
    fn set(&self, text: &str) -> Result<(), VaultError>;

    /// Clear the clipboard. On platforms where "clear" is not a distinct
    /// operation, implementations set an empty string.
    fn clear(&self) -> Result<(), VaultError>;
}
