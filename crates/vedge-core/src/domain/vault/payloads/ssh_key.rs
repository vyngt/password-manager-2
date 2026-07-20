use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::{
    expose_optional_secret_string, expose_secret_string,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    #[serde(serialize_with = "expose_secret_string")]
    pub private_key_pem: SecretString,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "expose_optional_secret_string"
    )]
    pub passphrase: Option<SecretString>,

    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
}
