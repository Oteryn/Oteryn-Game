//! Amount input is validated before any command is constructed.
pub fn parse_amount(text: &str) -> Option<u32> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|amount| (1..=100).contains(amount))
}
#[cfg(test)]
mod tests {
    use super::parse_amount;
    #[test]
    fn amount_requires_a_positive_in_range_integer() {
        for invalid in ["", "0", "-1", "1.5", "abc", "4294967296", "101"] {
            assert_eq!(parse_amount(invalid), None);
        }
        assert_eq!(parse_amount(" 7 "), Some(7));
        assert_eq!(parse_amount("100"), Some(100));
    }
}
