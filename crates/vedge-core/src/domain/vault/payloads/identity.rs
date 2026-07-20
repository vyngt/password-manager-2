use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::expose_optional_secret_string;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub line1: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,

    pub city: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    pub postal_code: String,

    /// ISO 3166-1 alpha-2 country code. Not validated at the domain layer.
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    pub first_name: String,
    pub last_name: String,
    pub email: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,

    /// ISO-8601 date (YYYY-MM-DD). Not validated here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub national_id: Option<SecretString>,
}
