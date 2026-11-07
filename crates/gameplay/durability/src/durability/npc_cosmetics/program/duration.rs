//! Crystal201189 `data/libs/systems/daily_reward.lua:62–83`, `string.diff`.
//! This preserves the donor's empty zero and floor/modulo behavior for negative values.
pub(super) fn source_duration(seconds: i64) -> String {
    let units = [
        ("day", seconds.div_euclid(86_400)),
        ("hour", seconds.div_euclid(3_600).rem_euclid(24)),
        ("minute", seconds.div_euclid(60).rem_euclid(60)),
        ("second", seconds.rem_euclid(60)),
    ];
    let mut result = String::new();
    for (name, value) in units {
        if value <= 0 {
            continue;
        }
        if name == "second" {
            result.push_str(" and ");
        } else if !result.is_empty() {
            result.push_str(", ");
        }
        result.push_str(&value.to_string());
        result.push(' ');
        result.push_str(name);
        if value != 1 {
            result.push('s');
        }
    }
    if result.len() < 16 && result.contains("second") {
        return result.strip_prefix(" and ").unwrap_or(&result).to_owned();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::source_duration;

    #[test]
    fn srv7_source_duration_retains_daily_reward_words_and_boundaries() {
        for (seconds, expected) in [
            (0, ""),
            (1, "1 second"),
            (2, "2 seconds"),
            (59, "59 seconds"),
            (60, "1 minute"),
            (61, "1 minute and 1 second"),
            (120, "2 minutes"),
            (3_600, "1 hour"),
            (3_661, "1 hour, 1 minute and 1 second"),
            (72_000, "20 hours"),
            (86_400, "1 day"),
            (86_401, "1 day and 1 second"),
            (172_800, "2 days"),
            (-1, "23 hours, 59 minutes and 59 seconds"),
            (-60, "23 hours, 59 minutes"),
            (-86_400, ""),
        ] {
            assert_eq!(source_duration(seconds), expected, "seconds={seconds}");
        }
    }
}
