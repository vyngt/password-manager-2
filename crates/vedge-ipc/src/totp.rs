//! TOTP wire types (slice 4.2).
//!
//! The seed does **not** cross to WASM via the payload DTO (the door). These
//! carry: a generated code ([`TotpCodeDto`]), a parsed enrolment echoed back to
//! the form ([`TotpEnrolmentDto`] — the seed the user just pasted, normalized),
//! and the inbound enrolment intent ([`TotpUpdateDto`]).

use serde::{Deserialize, Serialize};

/// HMAC algorithm on the wire — `PascalCase`, matching the domain enum and the
/// `otpauth://` `algorithm=` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TotpAlgorithmDto {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

/// A generated code + its remaining validity, from `reveal_totp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpCodeDto {
    pub code: String,
    pub seconds_remaining: u32,
    pub period: u32,
}

/// A parsed enrolment, echoed back to the form after `parse_totp_enrolment`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpEnrolmentDto {
    pub secret: String,
    pub algorithm: TotpAlgorithmDto,
    pub digits: u8,
    pub period: u32,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
}

/// Inbound enrolment intent for `update_entry`.
///
/// Mirrors `FieldSelectorDto`'s adjacent tagging. `Unchanged` is the serde
/// default, so a form that never touches TOTP preserves the stored seed — the
/// safe failure mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum TotpUpdateDto {
    #[default]
    Unchanged,
    Set(String),
    Clear,
}

#[cfg(test)]
mod tests {
    use super::{TotpAlgorithmDto, TotpCodeDto, TotpEnrolmentDto, TotpUpdateDto};

    #[test]
    fn totp_code_dto_round_trips() {
        let dto = TotpCodeDto {
            code: "492039".into(),
            seconds_remaining: 17,
            period: 30,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: TotpCodeDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code, "492039");
        assert_eq!(back.seconds_remaining, 17);
        assert_eq!(back.period, 30);
    }

    #[test]
    fn totp_enrolment_dto_round_trips_and_defaults() {
        // Partial JSON: optionals default to None.
        let dto: TotpEnrolmentDto = serde_json::from_str(
            r#"{"secret":"JBSWY3DPEHPK3PXP","algorithm":"Sha256","digits":8,"period":60}"#,
        )
        .unwrap();
        assert_eq!(dto.algorithm, TotpAlgorithmDto::Sha256);
        assert_eq!(dto.digits, 8);
        assert_eq!(dto.period, 60);
        assert!(dto.issuer.is_none());
        assert!(dto.account.is_none());

        let json = serde_json::to_string(&dto).unwrap();
        let back: TotpEnrolmentDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn totp_update_dto_adjacent_tagging_and_default() {
        assert!(matches!(TotpUpdateDto::default(), TotpUpdateDto::Unchanged));

        let set = TotpUpdateDto::Set("SEED".into());
        assert_eq!(
            serde_json::to_string(&set).unwrap(),
            r#"{"kind":"Set","value":"SEED"}"#
        );
        let back: TotpUpdateDto = serde_json::from_str(r#"{"kind":"Set","value":"SEED"}"#).unwrap();
        assert!(matches!(back, TotpUpdateDto::Set(s) if s == "SEED"));

        assert_eq!(
            serde_json::to_string(&TotpUpdateDto::Unchanged).unwrap(),
            r#"{"kind":"Unchanged"}"#
        );
    }
}
