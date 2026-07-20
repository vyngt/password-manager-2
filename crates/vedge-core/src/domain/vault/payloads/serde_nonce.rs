//! Base64 codec for the 24-byte blob nonce embedded in `DocumentPayload`.
//! Keeps JSON text-clean while preserving the exact byte value.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::vault::crypto_constants::NONCE_LEN;

pub fn serialize<S: Serializer>(nonce: &[u8; NONCE_LEN], ser: S) -> Result<S::Ok, S::Error> {
    STANDARD.encode(nonce).serialize(ser)
}

pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; NONCE_LEN], D::Error> {
    let s = String::deserialize(de)?;
    let bytes = STANDARD
        .decode(&s)
        .map_err(|e| serde::de::Error::custom(format!("invalid base64: {e}")))?;
    <[u8; NONCE_LEN]>::try_from(bytes.as_slice()).map_err(|_| {
        serde::de::Error::custom(format!("blob_nonce must be exactly {NONCE_LEN} bytes"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_24_byte_nonce() {
        let n: [u8; 24] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        ];
        let s = STANDARD.encode(n);
        let back: [u8; 24] = STANDARD.decode(&s).unwrap().try_into().unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn known_base64_vector() {
        assert_eq!(STANDARD.encode(b"Hello"), "SGVsbG8=");
        assert_eq!(STANDARD.decode("SGVsbG8=").unwrap(), b"Hello");
    }
}
