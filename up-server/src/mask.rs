use lazy_static::lazy_static;
use regex::Regex;

/// Mask ping key suitable for use in logs.
pub fn ping_key(key: &str) -> String {
    const DEFAULT_MASK: &str = "************";
    if key.len() < 4 {
        DEFAULT_MASK.to_string()
    } else {
        format!("{}{}", &key[0..4], DEFAULT_MASK)
    }
}

/// Mask email address suitable for use in logs.
pub fn email(address: &str) -> String {
    const DEFAULT_MASK: &str = "****@******";

    lazy_static! {
        static ref EMAIL_MASK_REGEX: Regex = Regex::new(r"^([^@]*)@(.+)$").unwrap();
    }

    if let Some(captures) = EMAIL_MASK_REGEX.captures(address) {
        match (captures.get(1), captures.get(2)) {
            (Some(address), Some(host)) => {
                let address = address.as_str();
                let host = host.as_str();
                let address = if !address.is_empty() {
                    &address[0..1]
                } else {
                    ""
                };
                let host = if host.len() > 2 {
                    &host[(host.len() - 2)..(host.len())]
                } else {
                    ""
                };
                format!("{}{}{}", address, DEFAULT_MASK, host)
            }
            _ => DEFAULT_MASK.to_string(),
        }
    } else {
        DEFAULT_MASK.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key_masking() {
        assert_eq!("************", ping_key(""));
        assert_eq!("************", ping_key("aaa"));
        assert_eq!("bbbb************", ping_key("bbbb"));
        assert_eq!("bbbb************", ping_key("bbbba"));
    }

    #[test]
    fn email_masking() {
        assert_eq!("x****@******zz", email("x@y.zz"));
        assert_eq!("h****@******om", email("hello@test.something.com"));
        assert_eq!("****@******", email("x@"));
        assert_eq!("****@******", email("@x"));
        assert_eq!("****@******", email("@"));
        assert_eq!("****@******", email(""));
    }
}
