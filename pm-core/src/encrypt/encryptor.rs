use magic_crypt::{new_magic_crypt, MagicCryptTrait};

pub fn encrypt_data(password: &str, text: &str) -> String {
    if text.len() == 0 {
        return String::from("");
    }

    let mc = new_magic_crypt!(password, 256);
    let ciphertext = mc.encrypt_str_to_base64(text);

    ciphertext
}

pub fn decrypt_data(password: &str, ciphertext: &str) -> String {
    if ciphertext.len() == 0 {
        return String::from("");
    }

    let mc = new_magic_crypt!(password, 256);
    let plaintext = mc
        .decrypt_base64_to_string(ciphertext)
        .unwrap_or_else(|_| String::from(""));

    plaintext
}
