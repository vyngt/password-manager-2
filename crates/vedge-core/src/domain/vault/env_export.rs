//! Env-var set export formatting (slice 5.4.1 ⑥).
//!
//! Renders an `EnvVars` entry's whole keyed set into either a `.env` file body
//! or a flat JSON object, for the set copy/reveal. Environment variables are a
//! `string → string` map — there is no value type, so JSON emits **every** value
//! as a string (see the spec's ⑥: a `value_type` would be fake fidelity that
//! evaporates in `.env` mode and silently corrupts leading zeros). Insertion
//! (vars) order is preserved in both formats.

use secrecy::ExposeSecret;
use zeroize::Zeroizing;

use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EnvVar;

/// Which shape the env-var set is copied / revealed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvExportFormat {
    /// `KEY='value'` lines, POSIX single-quoted so the value round-trips
    /// byte-exact AND inert (`$(…)`, backticks are literal). Cannot encode a
    /// value containing a newline.
    DotEnv,
    /// A flat `{"KEY":"value"}` object. Lossless — the format for values `.env`
    /// cannot represent.
    Json,
}

/// Wrap a value in POSIX single quotes, escaping each embedded `'` as `'\''`.
///
/// Inside single quotes every other byte — `$`, a backtick, `\`, spaces — is
/// literal, so the value round-trips byte-exact and is inert when the `.env`
/// file is sourced. Never strips a character (slice 5.3's ⑤: stripping corrupts
/// legitimate values).
fn sh_single_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(2));
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Format an env-var set. The output is secret plaintext, so it is wrapped in
/// `Zeroizing`.
///
/// - `DotEnv`: `KEY='value'` per line, in vars order. A value containing a
///   newline is rejected ([`VaultError::EnvValueNotDotEnvSafe`]) — `.env` cannot
///   encode it, and silently truncating on paste would be worse than an error.
///   The UI points the user at JSON.
/// - `Json`: a flat `{"KEY":"value"}` object (all values strings), keys in vars
///   order. Assembled by hand (not via a `serde_json::Map`, which sorts keys)
///   using `serde_json::to_string` to escape each key/value. Never
///   pretty-printed — it is clipboard-bound.
///
/// An empty set yields `""` (`.env`) or `"{}"` (JSON), not an error.
pub fn format_env_vars(
    vars: &[EnvVar],
    format: EnvExportFormat,
) -> Result<Zeroizing<String>, VaultError> {
    match format {
        EnvExportFormat::DotEnv => {
            let mut out = String::new();
            for (i, v) in vars.iter().enumerate() {
                let value = v.value.expose_secret();
                if value.contains('\n') || value.contains('\r') {
                    return Err(VaultError::EnvValueNotDotEnvSafe { key: v.key.clone() });
                }
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(&v.key);
                out.push('=');
                out.push_str(&sh_single_quote(value));
            }
            Ok(Zeroizing::new(out))
        }
        EnvExportFormat::Json => {
            let mut out = String::from("{");
            for (i, v) in vars.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let key = serde_json::to_string(&v.key)
                    .map_err(|e| VaultError::MalformedPayload(e.to_string()))?;
                let value = serde_json::to_string(v.value.expose_secret())
                    .map_err(|e| VaultError::MalformedPayload(e.to_string()))?;
                out.push_str(&key);
                out.push(':');
                out.push_str(&value);
            }
            out.push('}');
            Ok(Zeroizing::new(out))
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use secrecy::SecretString;

    fn ev(key: &str, value: &str) -> EnvVar {
        EnvVar {
            key: key.into(),
            value: SecretString::from(value),
        }
    }

    /// 🔴 ⑥ — `.env` single-quotes so a value round-trips byte-exact AND inert.
    /// `$(rm -rf ~)`, a backtick, and an embedded `'` all come back literally.
    #[test]
    fn dotenv_single_quotes_and_round_trips_byte_exact() {
        let vars = [
            ev("PLAIN", "just-a-value"),
            ev("INJECT", "$(rm -rf ~)"),
            ev("TICK", "a`whoami`b"),
            ev("QUOTE", "it's"),
        ];
        let out = format_env_vars(&vars, EnvExportFormat::DotEnv).unwrap();
        let expected = "PLAIN='just-a-value'\n\
             INJECT='$(rm -rf ~)'\n\
             TICK='a`whoami`b'\n\
             QUOTE='it'\\''s'";
        assert_eq!(&*out, expected);
        // The embedded `'` escape `'\''` re-parses to a literal apostrophe in
        // any POSIX shell / dotenv parser: 'it' + \' + 's' = it's.
    }

    /// A `.env` value is never stripped — a leading `=` (which the CSV adapter
    /// once mangled) survives verbatim inside the single quotes.
    #[test]
    fn dotenv_never_strips_a_leading_character() {
        let vars = [ev("K", "=leading-equals")];
        let out = format_env_vars(&vars, EnvExportFormat::DotEnv).unwrap();
        assert_eq!(&*out, "K='=leading-equals'");
    }

    /// 🔴 ⑥ — JSON is lossless where `.env` is not: a value with a newline
    /// round-trips through JSON, but the `.env` path errors (never truncates).
    #[test]
    fn json_is_lossless_where_dotenv_errors_on_newline() {
        let vars = [ev("CERT", "line1\nline2")];

        let err = format_env_vars(&vars, EnvExportFormat::DotEnv).unwrap_err();
        assert!(matches!(
            err,
            VaultError::EnvValueNotDotEnvSafe { ref key } if key == "CERT"
        ));

        let json = format_env_vars(&vars, EnvExportFormat::Json).unwrap();
        assert_eq!(&*json, r#"{"CERT":"line1\nline2"}"#);
    }

    /// JSON emits every value as a STRING (no `value_type`) and keeps vars order —
    /// a numeric-looking value with a leading zero is not coerced.
    #[test]
    fn json_all_strings_in_order_no_coercion() {
        let vars = [ev("PORT", "3000"), ev("PIN", "012345"), ev("DEBUG", "true")];
        let json = format_env_vars(&vars, EnvExportFormat::Json).unwrap();
        assert_eq!(&*json, r#"{"PORT":"3000","PIN":"012345","DEBUG":"true"}"#);
    }

    #[test]
    fn empty_set_is_not_an_error() {
        assert_eq!(&*format_env_vars(&[], EnvExportFormat::DotEnv).unwrap(), "");
        assert_eq!(&*format_env_vars(&[], EnvExportFormat::Json).unwrap(), "{}");
    }
}
