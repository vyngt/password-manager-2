//! TOTP DTO conversion layer (slice 4.2). The seed crosses only as an inbound
//! enrolment intent; the outbound direction exposes a presence flag + params +
//! the ephemeral code, never the seed.

pub use vedge_ipc::{TotpAlgorithmDto, TotpCodeDto, TotpEnrolmentDto, TotpUpdateDto};

use secrecy::{ExposeSecret, SecretString};

use vedge_core::{TotpAlgorithm, TotpCode, TotpEnrolment, TotpParams, TotpUpdate};

#[must_use]
pub const fn totp_algorithm_to_dto(a: TotpAlgorithm) -> TotpAlgorithmDto {
    match a {
        TotpAlgorithm::Sha1 => TotpAlgorithmDto::Sha1,
        TotpAlgorithm::Sha256 => TotpAlgorithmDto::Sha256,
        TotpAlgorithm::Sha512 => TotpAlgorithmDto::Sha512,
    }
}

#[must_use]
pub const fn totp_algorithm_from_dto(a: TotpAlgorithmDto) -> TotpAlgorithm {
    match a {
        TotpAlgorithmDto::Sha1 => TotpAlgorithm::Sha1,
        TotpAlgorithmDto::Sha256 => TotpAlgorithm::Sha256,
        TotpAlgorithmDto::Sha512 => TotpAlgorithm::Sha512,
    }
}

#[must_use]
pub const fn totp_params_from_dto(
    algorithm: TotpAlgorithmDto,
    digits: u8,
    period: u32,
) -> TotpParams {
    TotpParams {
        algorithm: totp_algorithm_from_dto(algorithm),
        digits,
        period,
    }
}

#[must_use]
pub fn totp_code_to_dto(c: &TotpCode) -> TotpCodeDto {
    TotpCodeDto {
        code: c.code.to_string(),
        seconds_remaining: c.seconds_remaining,
        period: c.period,
    }
}

#[must_use]
pub fn totp_enrolment_to_dto(e: &TotpEnrolment) -> TotpEnrolmentDto {
    TotpEnrolmentDto {
        secret: e.secret.expose_secret().to_owned(),
        algorithm: totp_algorithm_to_dto(e.params.algorithm),
        digits: e.params.digits,
        period: e.params.period,
        issuer: e.issuer.clone(),
        account: e.account.clone(),
    }
}

/// Inbound intent → domain. `Set` re-wraps the pasted seed into a `SecretString`
/// (it originated in WASM, so this is not a core→WASM disclosure).
#[must_use]
pub fn totp_update_from_dto(d: &TotpUpdateDto) -> TotpUpdate {
    match d {
        TotpUpdateDto::Unchanged => TotpUpdate::Unchanged,
        TotpUpdateDto::Set(s) => TotpUpdate::Set(SecretString::from(s.clone())),
        TotpUpdateDto::Clear => TotpUpdate::Clear,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{
        TotpAlgorithmDto, TotpUpdateDto, totp_algorithm_from_dto, totp_algorithm_to_dto,
        totp_update_from_dto,
    };
    use secrecy::ExposeSecret;
    use vedge_core::{TotpAlgorithm, TotpUpdate};

    #[test]
    fn algorithm_round_trips_all_variants() {
        for (dto, dom) in [
            (TotpAlgorithmDto::Sha1, TotpAlgorithm::Sha1),
            (TotpAlgorithmDto::Sha256, TotpAlgorithm::Sha256),
            (TotpAlgorithmDto::Sha512, TotpAlgorithm::Sha512),
        ] {
            assert_eq!(totp_algorithm_from_dto(dto), dom);
            assert_eq!(totp_algorithm_to_dto(dom), dto);
        }
    }

    #[test]
    fn update_intent_maps_all_variants() {
        assert!(matches!(
            totp_update_from_dto(&TotpUpdateDto::Unchanged),
            TotpUpdate::Unchanged
        ));
        assert!(matches!(
            totp_update_from_dto(&TotpUpdateDto::Clear),
            TotpUpdate::Clear
        ));
        let TotpUpdate::Set(s) = totp_update_from_dto(&TotpUpdateDto::Set("SEED".into())) else {
            panic!("expected Set");
        };
        assert_eq!(s.expose_secret(), "SEED");
    }
}
