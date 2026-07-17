//! Sealed-secret intent conversions (slice 5.4).
//!
//! Wire → domain only. `Set` re-wraps a WASM-originated `String` into a
//! `SecretString` (not a core→WASM disclosure). The outbound (domain → DTO)
//! direction always emits `Unchanged` (the sealed door), inlined in
//! `payload_to_dto`.

use secrecy::SecretString;

use vedge_core::{EnvVarUpdate, SecretListUpdate, SecretUpdate};
use vedge_ipc::{EnvVarUpdateDto, SecretListUpdateDto, SecretUpdateDto};

#[must_use]
pub fn secret_update_from_dto(d: SecretUpdateDto) -> SecretUpdate {
    match d {
        SecretUpdateDto::Unchanged => SecretUpdate::Unchanged,
        SecretUpdateDto::Set(s) => SecretUpdate::Set(SecretString::from(s)),
        SecretUpdateDto::Clear => SecretUpdate::Clear,
    }
}

#[must_use]
pub fn secret_list_update_from_dto(d: SecretListUpdateDto) -> SecretListUpdate {
    match d {
        SecretListUpdateDto::Unchanged => SecretListUpdate::Unchanged,
        SecretListUpdateDto::Set(v) => {
            SecretListUpdate::Set(v.into_iter().map(SecretString::from).collect())
        }
        SecretListUpdateDto::Clear => SecretListUpdate::Clear,
    }
}

#[must_use]
pub fn env_var_update_from_dto(d: EnvVarUpdateDto) -> EnvVarUpdate {
    EnvVarUpdate {
        key: d.key,
        value: secret_update_from_dto(d.value),
    }
}
