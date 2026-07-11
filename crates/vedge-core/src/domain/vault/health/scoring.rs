//! Health scoring — the zxcvbn wrapper.
//!
//! All values are tiny and bounded: the zxcvbn score is 0..=4, `guesses_log10`
//! a small `f64`, and scored inputs are capped at `MAX_SCORED_LEN` chars (zxcvbn
//! is superlinear — a longer password is trivially strong anyway). With those
//! bounds the workspace's denied float/int arithmetic lints are allowed here,
//! mirroring the accepted `vedge-generator/src/entropy.rs` and
//! `vedge-tauri/src/pdf/emergency_kit.rs` precedents.
#![allow(
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

/// zxcvbn is superlinear in password length; beyond this a password is trivially
/// strong, so we cap the scored input (in chars, to stay UTF-8-safe).
pub const MAX_SCORED_LEN: usize = 256;

/// A zxcvbn assessment: a 0..=4 score (maps straight into `PasswordStrengthMeter`)
/// and the base-10 log of the estimated guess count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeakScore {
    pub score: u8,
    pub guesses_log10: f64,
}

/// Score a secret against zxcvbn.
///
/// Biases with non-secret `user_inputs` (entry name, username, URL host) so
/// `"github2024"` is weak *for a GitHub entry*. The exposed plaintext is copied
/// only into a length-capped `Zeroizing` buffer.
#[must_use]
pub fn score(secret: &SecretString, user_inputs: &[&str]) -> WeakScore {
    let capped: Zeroizing<String> = Zeroizing::new(
        secret
            .expose_secret()
            .chars()
            .take(MAX_SCORED_LEN)
            .collect(),
    );
    let estimate = zxcvbn::zxcvbn(&capped, user_inputs);
    WeakScore {
        score: u8::from(estimate.score()),
        guesses_log10: estimate.guesses_log10(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obvious_weak_scores_low() {
        let s = SecretString::from("password");
        let assessed = score(&s, &[]);
        assert!(assessed.score <= 1, "score was {}", assessed.score);
    }

    #[test]
    fn strong_scores_high() {
        let s = SecretString::from("cr0ssRiver-Quartz!_92xzQ");
        let assessed = score(&s, &[]);
        assert!(assessed.score >= 3, "score was {}", assessed.score);
    }

    #[test]
    fn user_inputs_penalize_related_password() {
        // "github2024" is much weaker for an entry named github.
        let s = SecretString::from("github2024");
        let with = score(&s, &["github"]);
        assert!(with.score <= 2, "score was {}", with.score);
    }

    #[test]
    fn caps_scored_length() {
        // A 100k-char secret must not be handed to zxcvbn whole.
        let long = "a".repeat(100_000);
        let s = SecretString::from(long);
        let assessed = score(&s, &[]);
        // Just proves it returns without pathological cost; value unimportant.
        assert!(assessed.score <= 4);
    }
}
