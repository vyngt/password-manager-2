use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! ulid_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(Ulid::new().to_string())
            }

            pub fn from_raw(raw: impl Into<String>) -> Self {
                Self(raw.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

ulid_newtype!(EntryId);
ulid_newtype!(TagId);
ulid_newtype!(ThemeId);
ulid_newtype!(DeviceId);
ulid_newtype!(SessionId);

/// The vault **home** layout (slice 5.2.0). A vault is a `<name>.vedge/` directory
/// holding these three fixed, derivation-free members — no stems, nothing to get wrong.
pub const VAULT_FILE: &str = "vault.vdb";
pub const BLOBS_DIR: &str = "blobs";
pub const SNAPSHOTS_DIR: &str = "snapshots";

/// Identifies a vault by its **home directory** (`…/<name>.vedge`), slice 5.2.0. The
/// `.vdb` is an implementation detail of the home, reached via [`Self::vault_file`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VaultId(PathBuf);

impl VaultId {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// The home directory itself.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    #[must_use]
    pub fn into_path(self) -> PathBuf {
        self.0
    }

    /// The `SQLite` database file inside the home (`<home>/vault.vdb`). Fed to the repo
    /// factory / the read-only target reader — the only two direct openers of the `.vdb`.
    #[must_use]
    pub fn vault_file(&self) -> PathBuf {
        self.0.join(VAULT_FILE)
    }

    /// The blob directory inside the home (`<home>/blobs`).
    #[must_use]
    pub fn blobs_dir(&self) -> PathBuf {
        self.0.join(BLOBS_DIR)
    }

    /// The snapshots directory inside the home (`<home>/snapshots`; 5.2.1 fills it).
    #[must_use]
    pub fn snapshots_dir(&self) -> PathBuf {
        self.0.join(SNAPSHOTS_DIR)
    }
}

impl fmt::Display for VaultId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.display().to_string())
    }
}
