use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::{
    expose_optional_secret_string, expose_secret_string,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    pub cardholder_name: String,

    #[serde(serialize_with = "expose_secret_string")]
    pub number: SecretString,

    pub expiry_month: u8,
    pub expiry_year: u16,

    #[serde(serialize_with = "expose_secret_string")]
    pub cvv: SecretString,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub pin: Option<SecretString>,
}
