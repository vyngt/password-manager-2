use rand::Rng;
use std::iter;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "1234567890";
const SPECIAL: &str = "!\"#$%&'()*+,-.:;<=>?@[]^_`{|}~/\\";

#[tauri::command]
pub fn generate_password(
    len: i32,
    upper: bool,
    lower: bool,
    digits: bool,
    special: bool,
) -> String {
    let mut charset = String::from("");

    if upper {
        charset.push_str(UPPERCASE);
    }

    if lower {
        charset.push_str(LOWERCASE)
    }

    if digits {
        charset.push_str(DIGITS);
    }

    if special {
        charset.push_str(SPECIAL);
    }

    generate(charset.as_str(), len)
}

pub fn generate(charset: &str, len: i32) -> String {
    if *&charset.len() == 0 && len == 0 {
        return String::from("");
    }

    let bytes_charset = charset.as_bytes();
    let mut rng = rand::thread_rng();
    let one_char = || bytes_charset[rng.gen_range(0..bytes_charset.len())] as char;
    iter::repeat_with(one_char).take(len as usize).collect()
}
