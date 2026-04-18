use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    Login,
    Card,
    SshKey,
    ApiKey,
    EnvVars,
    Note,
    Document,
    Identity,
    Folder,
    #[serde(untagged)]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_variants_serialize_as_pascal_case() {
        assert_eq!(serde_json::to_string(&EntryType::Login).unwrap(), "\"Login\"");
        assert_eq!(serde_json::to_string(&EntryType::SshKey).unwrap(), "\"SshKey\"");
        assert_eq!(serde_json::to_string(&EntryType::EnvVars).unwrap(), "\"EnvVars\"");
        assert_eq!(serde_json::to_string(&EntryType::ApiKey).unwrap(), "\"ApiKey\"");
    }

    #[test]
    fn unknown_variant_round_trips() {
        let v: EntryType = serde_json::from_str("\"Passkey\"").unwrap();
        assert_eq!(v, EntryType::Unknown("Passkey".into()));
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"Passkey\"");
    }

    #[test]
    fn known_deserializes_as_known() {
        let v: EntryType = serde_json::from_str("\"Login\"").unwrap();
        assert_eq!(v, EntryType::Login);
    }
}
