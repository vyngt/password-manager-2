use password_hash::{rand_core::OsRng, PasswordHash, PasswordVerifier, SaltString};

use argon2::{Argon2, PasswordHasher};
// use pbkdf2::pbkdf2;

pub fn set_master_password(password: &str) {
    let b_pw = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default().hash_password(b_pw, &salt);

    let result = match hashed {
        Ok(value) => value.to_string(),
        Err(_e) => String::from(""),
    };
}

pub fn check_identity(password: &str, password_hashed: &str) -> bool {
    let b_pw = password.as_bytes();
    let parsed_hash = PasswordHash::new(&password_hashed);
    let parsed_hash = match parsed_hash {
        Ok(ph) => ph,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(b_pw, &parsed_hash)
        .is_ok()
}
