//! The sealed export envelope (slice 5.3a).
//!
//! ```text
//! [magic 8][format_version u32][kdf_id u8][m u32][t u32][p u32][salt 16][nonce 24] ‖ XChaCha20-Poly1305(plaintext)
//! └──────────────────────── prefix (the AEAD's AAD) ────────────────────────┘
//! ```
//!
//! # Why an envelope at all (Decision ②)
//!
//! 5.2's backup was relaxed about memory because its payload was already
//! ciphertext end to end. **Export is the inverse: the tar inside is PLAINTEXT**
//! — real passwords, keys, card numbers. This envelope is the only thing between
//! them and the disk. It seals in memory (`seal` takes bytes, returns bytes); the
//! plaintext tar never touches disk (the caller writes only the sealed output).
//!
//! # The header is self-describing and the guard is `>` (Decision ⑥)
//!
//! The Argon2id parameters (`m`/`t`/`p`) and `salt` live in the file, so a future
//! or weaker build reads them from the artifact — old exports stay openable if
//! the defaults ever move. The version guard **refuses newer, accepts older,
//! forever** (`>`, never `!=` — H1 has shipped twice this phase). Fixed
//! conservative params (RFC 9106 memory-constrained, `p=1` because WASM is
//! single-threaded), never calibrate-on-create — a desktop-calibrated artifact
//! would be hostile to a phone.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::{Rng, rng};
use zeroize::Zeroizing;

use crate::domain::vault::crypto_constants::NONCE_LEN;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::crypto::xchacha20::{aead_decrypt, aead_encrypt};

/// `b"VEDGEXPT"` — VEDGE eXPorT. First 8 bytes of every export file.
pub const ENVELOPE_MAGIC: &[u8; 8] = b"VEDGEXPT";

/// Bumped only on a breaking change to the envelope framing. Read with `>`.
pub const ENVELOPE_FORMAT_VERSION: u32 = 1;

const KDF_ARGON2ID: u8 = 1;

/// Fixed Argon2id profile: RFC 9106's memory-constrained second profile
/// (64 MiB, t=3), parallelism dropped to 1 (WASM is single-threaded; determinism
/// over lanes). Well above OWASP's floor; opens fine on a phone.
const ARGON2_M_KIB: u32 = 65_536;
const ARGON2_T: u32 = 3;
const ARGON2_P: u32 = 1;

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

// Fixed field offsets within the prefix (the AAD). Const arithmetic only.
const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = OFF_MAGIC + 8;
const OFF_KDF: usize = OFF_VERSION + 4;
const OFF_M: usize = OFF_KDF + 1;
const OFF_T: usize = OFF_M + 4;
const OFF_P: usize = OFF_T + 4;
const OFF_SALT: usize = OFF_P + 4;
const PREFIX_LEN: usize = OFF_SALT + SALT_LEN; // 41
const HEADER_LEN: usize = PREFIX_LEN + NONCE_LEN; // 65

fn malformed(msg: &str) -> VaultError {
    VaultError::ExportMalformed(msg.to_owned())
}

/// Derive a 32-byte envelope key straight from the passphrase (no 2SKD, no Secret
/// Key) with the header's Argon2id params.
fn derive_key(
    passphrase: &[u8],
    salt: &[u8],
    m: u32,
    t: u32,
    p: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, VaultError> {
    let params = Params::new(m, t, p, Some(KEY_LEN))
        .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    a2.hash_password_into(passphrase, salt, key.as_mut_slice())
        .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
    Ok(key)
}

fn build_prefix(version: u32, m: u32, t: u32, p: u32, salt: &[u8; SALT_LEN]) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(PREFIX_LEN);
    prefix.extend_from_slice(ENVELOPE_MAGIC);
    prefix.extend_from_slice(&version.to_le_bytes());
    prefix.push(KDF_ARGON2ID);
    prefix.extend_from_slice(&m.to_le_bytes());
    prefix.extend_from_slice(&t.to_le_bytes());
    prefix.extend_from_slice(&p.to_le_bytes());
    prefix.extend_from_slice(salt);
    prefix
}

/// Seal `plaintext` under `passphrase` with an explicit version + Argon2 profile.
/// Production seals go through [`seal`]; this exists so tests can forge a
/// future-version or weaker-param artifact for the version-guard canary.
fn seal_with(
    passphrase: &[u8],
    plaintext: &[u8],
    version: u32,
    m: u32,
    t: u32,
    p: u32,
) -> Result<Vec<u8>, VaultError> {
    let mut salt = [0u8; SALT_LEN];
    rng().fill_bytes(&mut salt);

    let key = derive_key(passphrase, &salt, m, t, p)?;
    let prefix = build_prefix(version, m, t, p, &salt);

    // AAD = the prefix, so the params/version/salt cannot be swapped without the
    // tag failing. The nonce need not be in the AAD — tampering with it fails
    // decryption anyway.
    let (nonce, ciphertext) = aead_encrypt(&key, plaintext, &prefix)?;

    let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(ciphertext.len()));
    out.extend_from_slice(&prefix);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Seal `plaintext` (a plaintext tar of the export) under `passphrase`.
///
/// Returns the complete self-describing envelope bytes. The output is ciphertext
/// — safe to write to disk. The plaintext never leaves memory here.
pub fn seal(passphrase: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    seal_with(
        passphrase,
        plaintext,
        ENVELOPE_FORMAT_VERSION,
        ARGON2_M_KIB,
        ARGON2_T,
        ARGON2_P,
    )
}

fn read_u32(buf: &[u8], off: usize) -> Result<u32, VaultError> {
    let end = off
        .checked_add(4)
        .ok_or_else(|| malformed("header offset overflow"))?;
    let slice = buf
        .get(off..end)
        .ok_or_else(|| malformed("header truncated"))?;
    let arr: [u8; 4] = slice
        .try_into()
        .map_err(|_| malformed("header truncated"))?;
    Ok(u32::from_le_bytes(arr))
}

/// Open an export envelope, returning the plaintext tar bytes (zeroizing).
///
/// Refuses a `format_version` newer than this build (`>`), a bad magic, or an
/// unknown KDF id. A wrong passphrase (or any tampering) collapses to
/// [`VaultError::DecryptionFailed`].
pub fn open(passphrase: &[u8], envelope: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let prefix = envelope
        .get(..PREFIX_LEN)
        .ok_or_else(|| malformed("file is too short to be a VEdge export"))?;

    let magic = prefix
        .get(OFF_MAGIC..OFF_VERSION)
        .ok_or_else(|| malformed("header truncated"))?;
    if magic != ENVELOPE_MAGIC {
        return Err(malformed("not a VEdge export (bad magic)"));
    }

    let version = read_u32(prefix, OFF_VERSION)?;
    // 🔴 H1: refuse NEWER, accept OLDER — forever.
    if version > ENVELOPE_FORMAT_VERSION {
        return Err(VaultError::ExportUnsupportedFormat(version));
    }

    let kdf_id = *prefix
        .get(OFF_KDF)
        .ok_or_else(|| malformed("header truncated"))?;
    if kdf_id != KDF_ARGON2ID {
        return Err(malformed("unknown key-derivation id"));
    }

    let m = read_u32(prefix, OFF_M)?;
    let t = read_u32(prefix, OFF_T)?;
    let p = read_u32(prefix, OFF_P)?;
    let salt = prefix
        .get(OFF_SALT..PREFIX_LEN)
        .ok_or_else(|| malformed("header truncated"))?;

    let nonce_slice = envelope
        .get(PREFIX_LEN..HEADER_LEN)
        .ok_or_else(|| malformed("header truncated"))?;
    let nonce: [u8; NONCE_LEN] = nonce_slice
        .try_into()
        .map_err(|_| malformed("header truncated"))?;
    let ciphertext = envelope
        .get(HEADER_LEN..)
        .ok_or_else(|| malformed("missing ciphertext"))?;

    let key = derive_key(passphrase, salt, m, t, p)?;
    aead_decrypt(&key, &nonce, ciphertext, prefix)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )]

    use super::*;

    // Fast, valid Argon2 params for the round-trip tests (production is 64 MiB).
    const FAST_M: u32 = 32;
    const FAST_T: u32 = 1;
    const FAST_P: u32 = 1;

    fn seal_fast(pw: &[u8], pt: &[u8]) -> Vec<u8> {
        seal_with(pw, pt, ENVELOPE_FORMAT_VERSION, FAST_M, FAST_T, FAST_P).unwrap()
    }

    #[test]
    fn seals_and_opens_round_trip() {
        let pt = b"the plaintext tar bytes with secrets";
        let sealed = seal_fast(b"correct horse", pt);
        let opened = open(b"correct horse", &sealed).unwrap();
        assert_eq!(&*opened, pt);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let sealed = seal_fast(b"right", b"secret data");
        let err = open(b"wrong", &sealed).unwrap_err();
        assert!(matches!(err, VaultError::DecryptionFailed));
    }

    #[test]
    fn output_is_opaque() {
        let pt = b"password=hunter2 THIS_MUST_NOT_APPEAR_IN_CIPHERTEXT";
        let sealed = seal_fast(b"pw", pt);
        // The plaintext must not appear anywhere in the sealed bytes.
        assert!(
            !sealed.windows(pt.len()).any(|w| w == pt),
            "plaintext leaked into the envelope"
        );
        // Nor a distinctive secret substring.
        let needle = b"hunter2";
        assert!(!sealed.windows(needle.len()).any(|w| w == needle));
    }

    #[test]
    fn magic_and_version_are_in_the_header() {
        let sealed = seal_fast(b"pw", b"x");
        assert_eq!(&sealed[OFF_MAGIC..OFF_VERSION], ENVELOPE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(sealed[OFF_VERSION..OFF_KDF].try_into().unwrap()),
            ENVELOPE_FORMAT_VERSION
        );
    }

    /// 🔴 Decision ⑥ canary — the third time this guard has been written in Phase 5.
    /// A NEWER `format_version` is refused; an OLDER one opens forever.
    #[test]
    fn refuses_newer_version_accepts_older() {
        // Forge a future-version artifact, internally consistent (AAD includes the
        // version, so it must be sealed AT that version, not patched afterward).
        let future = seal_with(
            b"pw",
            b"data",
            ENVELOPE_FORMAT_VERSION + 1,
            FAST_M,
            FAST_T,
            FAST_P,
        )
        .unwrap();
        let err = open(b"pw", &future).unwrap_err();
        assert!(
            matches!(err, VaultError::ExportUnsupportedFormat(v) if v == ENVELOPE_FORMAT_VERSION + 1),
            "a newer export must be refused, got {err:?}"
        );

        // An older artifact (version 0) must still open — accept older, forever.
        let older = seal_with(b"pw", b"data", 0, FAST_M, FAST_T, FAST_P).unwrap();
        let opened = open(b"pw", &older).unwrap();
        assert_eq!(&*opened, b"data");
    }

    /// 🔴 Test #5 — self-describing params: an artifact forged with weaker memory
    /// opens on a build whose default is 64 MiB, because `open` reads m/t/p from
    /// the header, not from the build's constants.
    #[test]
    fn self_describing_params_open_regardless_of_build_default() {
        // 19 MiB — OWASP's floor, deliberately not the 64 MiB build default.
        let sealed = seal_with(b"pw", b"data", ENVELOPE_FORMAT_VERSION, 19_456, 2, 1).unwrap();
        // The header must carry those exact params...
        assert_eq!(
            u32::from_le_bytes(sealed[OFF_M..OFF_T].try_into().unwrap()),
            19_456
        );
        // ...and `open` must honor them and succeed.
        let opened = open(b"pw", &sealed).unwrap();
        assert_eq!(&*opened, b"data");
    }

    #[test]
    fn rejects_bad_magic_and_short_input() {
        assert!(matches!(
            open(b"pw", b"tiny").unwrap_err(),
            VaultError::ExportMalformed(_)
        ));
        let mut sealed = seal_fast(b"pw", b"data");
        sealed[0] = b'X'; // corrupt the magic
        assert!(matches!(
            open(b"pw", &sealed).unwrap_err(),
            VaultError::ExportMalformed(_)
        ));
    }
}
