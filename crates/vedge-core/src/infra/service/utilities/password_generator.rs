use crate::business::domain::services::GeneratePasswordOptions;
use rand::Rng;
use std::iter;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "1234567890";
const SPECIAL: &str = "!\"#$%&'()*+,-.:;<=>?@[]^_`{|}~/\\";

pub fn generate_password(opt: GeneratePasswordOptions) -> String {
    let mut charset = String::from("");

    if opt.upper {
        charset.push_str(UPPERCASE);
    }

    if opt.lower {
        charset.push_str(LOWERCASE)
    }

    if opt.digits {
        charset.push_str(DIGITS);
    }

    if opt.special {
        charset.push_str(SPECIAL);
    }

    generate(charset.as_str(), opt.len)
}

fn generate(charset: &str, len: i32) -> String {
    if *&charset.len() == 0 || len == 0 {
        return String::from("");
    }

    let bytes_charset = charset.as_bytes();
    let mut rng = rand::rng();
    let one_char = || bytes_charset[rng.random_range(0..bytes_charset.len())] as char;
    iter::repeat_with(one_char).take(len as usize).collect()
}
