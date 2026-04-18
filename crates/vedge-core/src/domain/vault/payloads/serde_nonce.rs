//! Base64 codec for the 24-byte blob nonce embedded in `DocumentPayload`.
//! Keeps JSON text-clean while preserving the exact byte value.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::vault::crypto_constants::NONCE_LEN;

const ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PAD: u8 = b'=';

fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let chunks = input.chunks_exact(3);
    let remainder = chunks.remainder();
    for c in chunks {
        let n = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
    }
    match remainder.len() {
        0 => {}
        1 => {
            let n = (remainder[0] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push(PAD as char);
            out.push(PAD as char);
        }
        2 => {
            let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
            out.push(PAD as char);
        }
        _ => unreachable!(),
    }
    out
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err("base64 length must be multiple of 4".into());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let decode = |b: u8| -> Result<u32, String> {
        match b {
            b'A'..=b'Z' => Ok((b - b'A') as u32),
            b'a'..=b'z' => Ok((b - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((b - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("invalid base64 byte: {b:?}")),
        }
    };
    for chunk in bytes.chunks_exact(4) {
        let [a, b, c, d] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        let n = (decode(a)? << 18)
            | (decode(b)? << 12)
            | (if c == PAD { 0 } else { decode(c)? << 6 })
            | (if d == PAD { 0 } else { decode(d)? });
        out.push(((n >> 16) & 0xFF) as u8);
        if c != PAD {
            out.push(((n >> 8) & 0xFF) as u8);
        }
        if d != PAD {
            out.push((n & 0xFF) as u8);
        }
    }
    Ok(out)
}

pub fn serialize<S: Serializer>(nonce: &[u8; NONCE_LEN], ser: S) -> Result<S::Ok, S::Error> {
    encode_base64(nonce).serialize(ser)
}

pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; NONCE_LEN], D::Error> {
    let s = String::deserialize(de)?;
    let bytes = decode_base64(&s).map_err(serde::de::Error::custom)?;
    <[u8; NONCE_LEN]>::try_from(bytes.as_slice()).map_err(|_| {
        serde::de::Error::custom(format!(
            "blob_nonce must be exactly {NONCE_LEN} bytes"
        ))
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
        let s = encode_base64(&n);
        let back = decode_base64(&s).unwrap();
        assert_eq!(back.as_slice(), &n);
    }

    #[test]
    fn known_base64_vector() {
        // "Hello" → "SGVsbG8="
        assert_eq!(encode_base64(b"Hello"), "SGVsbG8=");
        assert_eq!(decode_base64("SGVsbG8=").unwrap(), b"Hello");
    }
}
