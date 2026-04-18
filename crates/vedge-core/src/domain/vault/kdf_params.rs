use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub alg: String,
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub version: u32,
}

impl KdfParams {
    #[must_use] 
    pub fn argon2id_default() -> Self {
        Self {
            alg: "argon2id".to_owned(),
            m: 262_144,
            t: 3,
            p: 4,
            version: 1,
        }
    }
}
