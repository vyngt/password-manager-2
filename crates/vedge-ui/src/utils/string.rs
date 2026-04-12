use std::collections::HashSet;

pub fn merge_classnames<I, S>(inputs: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    let mut result: Vec<String> = Vec::new();

    for input in inputs {
        let s = input.as_ref();
        let parts = s.split_whitespace();
        for part in parts {
            if !seen.contains(&part.to_string()) {
                seen.insert(part.to_string());
                result.push(part.to_string());
            }
        }
    }

    result.join(" ")
}
