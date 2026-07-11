use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::{
    expose_optional_secret_string, expose_secret_string, expose_secret_string_vec,
};
use crate::domain::vault::totp::TotpParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    pub username: String,

    #[serde(serialize_with = "expose_secret_string")]
    pub password: SecretString,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub totp_secret: Option<SecretString>,

    /// TOTP parameters for `totp_secret` (slice 4.2). Stored alongside the seed
    /// so non-default issuers (SHA-256, 8 digits, 60 s) survive round-trips.
    /// `#[serde(default)]` keeps pre-4.2 payloads (which lack the field)
    /// deserializing as the SHA-1 / 6 / 30 defaults.
    #[serde(default)]
    pub totp_params: TotpParams,

    #[serde(default, serialize_with = "expose_secret_string_vec")]
    pub recovery_codes: Vec<SecretString>,
}
