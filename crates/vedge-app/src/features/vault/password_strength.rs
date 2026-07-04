//! Lightweight password-strength scoring (0–4) for the onboarding meter.
//!
//! Heuristic only — length + character-class variety. Not a real estimator
//! like zxcvbn; it just nudges users toward stronger passwords and feeds the
//! `PasswordStrengthMeter` (which visualizes a caller-computed score).

/// Score a password 0 (empty) → 4 (strong).
#[must_use]
pub fn score(password: &str) -> u8 {
    if password.is_empty() {
        return 0;
    }

    let len = password.chars().count();
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
    let classes =
        u8::from(has_lower) + u8::from(has_upper) + u8::from(has_digit) + u8::from(has_symbol);

    let mut s: u8 = 0;
    if len >= 8 {
        s += 1;
    }
    if len >= 12 {
        s += 1;
    }
    if classes >= 2 {
        s += 1;
    }
    if classes >= 3 && len >= 10 {
        s += 1;
    }
    s.min(4)
}
