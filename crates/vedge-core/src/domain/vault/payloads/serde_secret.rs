//! Serialization helpers that **expose** `SecretString` values to serde.
//!
//! These helpers are the only JSON serialization path for secret fields.
//! Every call site is intentional: the output bytes are immediately fed to
//! `CryptoProvider::encrypt_entry` and then wiped. Grep for
//! `expose_secret_string` to audit every encryption boundary.
//!
//! Deserialization remains automatic via `secrecy`'s built-in
//! `Deserialize` impl — safe, produces a zeroizing wrapper.

use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, Serializer};

pub fn expose_secret_string<S: Serializer>(
    secret: &SecretString,
    ser: S,
) -> Result<S::Ok, S::Error> {
    secret.expose_secret().serialize(ser)
}

pub fn expose_optional_secret_string<S: Serializer>(
    secret: &Option<SecretString>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    match secret {
        Some(s) => ser.serialize_some(s.expose_secret()),
        None => ser.serialize_none(),
    }
}

pub fn expose_secret_string_vec<S: Serializer>(
    secrets: &[SecretString],
    ser: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = ser.serialize_seq(Some(secrets.len()))?;
    for s in secrets {
        seq.serialize_element(s.expose_secret())?;
    }
    seq.end()
}
