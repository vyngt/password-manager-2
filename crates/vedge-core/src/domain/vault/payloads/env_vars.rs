use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::domain::vault::payloads::common_meta::CommonMeta;
use crate::domain::vault::payloads::serde_secret::expose_secret_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,

    #[serde(serialize_with = "expose_secret_string")]
    pub value: SecretString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarsPayload {
    #[serde(flatten)]
    pub meta: CommonMeta,

    #[serde(default)]
    pub vars: Vec<EnvVar>,
}
