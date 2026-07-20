use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::{
    expose_optional_secret_string, expose_secret_string, expose_secret_string_vec,
};

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

    #[serde(default, serialize_with = "expose_secret_string_vec")]
    pub recovery_codes: Vec<SecretString>,
}
