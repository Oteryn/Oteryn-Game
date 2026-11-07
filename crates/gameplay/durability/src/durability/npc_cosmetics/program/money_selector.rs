//! Crystal201 data/libs/functions/functions.lua115–122,284–286.
//! First ASCII digit run; the source ignores sign/suffix and rejects invalid money.
pub(super) fn selected_money(message: &str) -> Option<i64> {
    let bytes = message.as_bytes();
    let start = bytes.iter().position(u8::is_ascii_digit)?;
    let count = bytes[start..]
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let value = message[start..start + count].parse::<i64>().ok()?;
    (value > 0 && value < 9_007_199_254_740_992).then_some(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_source_money_selector_uses_first_ascii_run_and_valid_money_bounds() {
        for (text, expected) in [
            ("level 3", Some(3)),
            ("-6", Some(6)),
            ("6k", Some(6)),
            ("x12 y3", Some(12)),
            ("6", Some(6)),
            ("0", None),
            ("none", None),
            ("9007199254740991", Some(9_007_199_254_740_991)),
            ("9007199254740992", None),
            ("999999999999999999999999", None),
            ("６", None),
            ("x0 y6", None),
        ] {
            assert_eq!(selected_money(text), expected, "source input {text:?}");
        }
    }
}
