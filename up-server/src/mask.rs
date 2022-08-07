/// Mask ping key suitable for use in logs.
pub fn ping_key(s: &str) -> String {
    format!("{}************", &s[0..4])
}
