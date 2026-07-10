//! Pattern mode — a token grammar over character classes with literal escapes.
//! Honest entropy `Σ log2(pool)` over the non-literal tokens; literal positions
//! contribute 0. A pattern like `aaaa-9999` is `4·log2(26) + 4·log2(10) ≈ 32`
//! bits — far less than 9 random chars, and shown truthfully.
//!
//! Grammar (documented token table):
//! - `a` → lowercase, `A` → uppercase, `9` → digit, `#` → symbol, `*` → any
//!   (the full charset)
//! - `\x` → literal `x` (escape), any other char → literal
//!
//! The parser runs **once**; both generate and preview consume the same token
//! stream, so entropy can never disagree with what was drawn.

use crate::charset::{class_all, class_digit, class_lower, class_symbol, class_upper};
use crate::{GenError, GeneratedSecret, MAX_LEN, entropy};
use rand::Rng;
use rand::seq::IndexedRandom;
use zeroize::Zeroizing;

/// Configuration for the pattern mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternConfig {
    /// The token grammar string (see module docs).
    pub pattern: String,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            pattern: "Aaaaaa-9999".to_owned(),
        }
    }
}

/// One parsed position: a uniform draw from a pool, or a fixed literal char.
enum Token {
    Random(Vec<char>),
    Literal(char),
}

/// Parse the grammar into a token stream, validating as it goes.
///
/// Errors, in the order they are detected:
/// - [`GenError::InvalidPattern`] — a dangling escape (trailing `\`).
/// - [`GenError::EmptyPattern`] — no random token at all (empty string or every
///   token a literal): entropy 0 is not a "secret".
/// - [`GenError::LengthOutOfRange`] — more than [`MAX_LEN`] output positions.
fn parse_pattern(pattern: &str) -> Result<Vec<Token>, GenError> {
    let mut tokens = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        let token = match c {
            '\\' => match chars.next() {
                Some(lit) => Token::Literal(lit),
                None => return Err(GenError::InvalidPattern),
            },
            'a' => Token::Random(class_lower()),
            'A' => Token::Random(class_upper()),
            '9' => Token::Random(class_digit()),
            '#' => Token::Random(class_symbol()),
            '*' => Token::Random(class_all()),
            other => Token::Literal(other),
        };
        tokens.push(token);
    }
    if !tokens.iter().any(|t| matches!(t, Token::Random(_))) {
        return Err(GenError::EmptyPattern);
    }
    // Each token emits exactly one output char, so token count == output length.
    if tokens.len() > MAX_LEN as usize {
        return Err(GenError::LengthOutOfRange);
    }
    Ok(tokens)
}

/// `Σ log2(pool)` over the random tokens; literals are omitted (0 bits).
fn token_entropy(tokens: &[Token]) -> f64 {
    let sizes: Vec<usize> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Random(pool) => Some(pool.len()),
            Token::Literal(_) => None,
        })
        .collect();
    entropy::log2_sum(&sizes)
}

/// Draw each random token from its pool; pass literals through unchanged.
fn generate_from_tokens<R: Rng>(tokens: &[Token], rng: &mut R) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(tokens.len()));
    for t in tokens {
        match t {
            Token::Random(pool) => {
                if let Some(&c) = pool.choose(rng) {
                    out.push(c);
                }
            }
            Token::Literal(c) => out.push(*c),
        }
    }
    out
}

/// Entropy preview: `Σ log2(pool)` over the parsed non-literal tokens.
///
/// # Errors
/// See [`parse_pattern`].
pub fn pattern_entropy(cfg: &PatternConfig) -> Result<f64, GenError> {
    let tokens = parse_pattern(&cfg.pattern)?;
    Ok(token_entropy(&tokens))
}

/// Generate from the pattern. Parses once and derives entropy from the same
/// token stream it draws from.
pub fn pattern_with<R: Rng>(cfg: &PatternConfig, rng: &mut R) -> Result<GeneratedSecret, GenError> {
    let tokens = parse_pattern(&cfg.pattern)?;
    let entropy_bits = token_entropy(&tokens);
    let secret = generate_from_tokens(&tokens, rng);
    Ok(GeneratedSecret {
        secret,
        entropy_bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenError;
    use crate::charset::{class_all, class_symbol};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn cfg(p: &str) -> PatternConfig {
        PatternConfig {
            pattern: p.to_owned(),
        }
    }

    #[test]
    fn pattern_entropy_closed_form() {
        let bits = pattern_entropy(&cfg("AAAA9999")).unwrap();
        assert!((bits - (4.0 * 26.0_f64.log2() + 4.0 * 10.0_f64.log2())).abs() < 1e-9);

        let bits = pattern_entropy(&cfg("****")).unwrap();
        let all = class_all().len() as f64;
        assert!((bits - 4.0 * all.log2()).abs() < 1e-9);

        let bits = pattern_entropy(&cfg("#")).unwrap();
        assert!((bits - (class_symbol().len() as f64).log2()).abs() < 1e-9);
    }

    #[test]
    fn pattern_preview_matches_generate() {
        let c = cfg("aA9#-*");
        let mut rng = StdRng::seed_from_u64(4);
        assert_eq!(
            pattern_entropy(&c).unwrap(),
            pattern_with(&c, &mut rng).unwrap().entropy_bits
        );
    }

    #[test]
    fn pattern_tokens_and_literals() {
        let mut rng = StdRng::seed_from_u64(6);
        let out = pattern_with(&cfg("aA9#-"), &mut rng).unwrap();
        let cs: Vec<char> = out.secret.chars().collect();
        assert_eq!(cs.len(), 5);
        assert!(cs[0].is_ascii_lowercase());
        assert!(cs[1].is_ascii_uppercase());
        assert!(cs[2].is_ascii_digit());
        assert!(class_symbol().contains(&cs[3]));
        assert_eq!(cs[4], '-');
    }

    #[test]
    fn pattern_escape_yields_literal() {
        // `\A` → literal 'A'; the trailing `a` is a random lowercase token.
        let mut rng = StdRng::seed_from_u64(8);
        let out = pattern_with(&cfg("\\Aa"), &mut rng).unwrap();
        let cs: Vec<char> = out.secret.chars().collect();
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0], 'A');
        assert!(cs[1].is_ascii_lowercase());
    }

    #[test]
    fn pattern_unicode_literal() {
        let mut rng = StdRng::seed_from_u64(10);
        let out = pattern_with(&cfg("éa"), &mut rng).unwrap();
        let cs: Vec<char> = out.secret.chars().collect();
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0], 'é');
        assert!(cs[1].is_ascii_lowercase());
    }

    #[test]
    fn pattern_errors() {
        assert_eq!(
            pattern_entropy(&cfg("")).err(),
            Some(GenError::EmptyPattern)
        );
        assert_eq!(
            pattern_entropy(&cfg("----")).err(),
            Some(GenError::EmptyPattern)
        );
        assert_eq!(
            pattern_entropy(&cfg("ab\\")).err(),
            Some(GenError::InvalidPattern)
        );
        let long = "a".repeat(129);
        assert_eq!(
            pattern_entropy(&cfg(&long)).err(),
            Some(GenError::LengthOutOfRange)
        );
        // All-literal + too long → EmptyPattern (empty is checked before length).
        let long_literal = "-".repeat(200);
        assert_eq!(
            pattern_entropy(&cfg(&long_literal)).err(),
            Some(GenError::EmptyPattern)
        );
    }
}
