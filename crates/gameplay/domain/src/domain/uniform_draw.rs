//! Shared deterministic bounded draw arithmetic.

/// Uniform draw in `[minimum, maximum]` from one deterministic decision value. Canary and Crystal
/// draw with `normal_random`; the Reference distribution is not evidenced, so this stays a
/// replaceable world rule.
pub fn uniform_draw(decision: u64, minimum: i64, maximum: i64) -> i64 {
    if maximum <= minimum {
        return minimum;
    }
    let span = (i128::from(maximum) - i128::from(minimum) + 1) as u128;
    let offset = (u128::from(decision) % span) as i128;
    // The span is at most 2^64; adding an offset below it stays within the i64 bounds.
    (i128::from(minimum) + offset) as i64
}

#[cfg(test)]
mod tests {
    use super::uniform_draw;

    #[test]
    fn draws_across_full_i64_range() {
        for (decision, expected) in [
            (0, i64::MIN),
            (i64::MAX as u64, -1),
            (1_u64 << 63, 0),
            (u64::MAX, i64::MAX),
        ] {
            assert_eq!(uniform_draw(decision, i64::MIN, i64::MAX), expected);
        }
    }

    #[test]
    fn wide_ranges_preserve_inclusive_span_and_large_offsets() {
        assert_eq!(
            uniform_draw(u64::MAX - 1, i64::MIN, i64::MAX - 1),
            i64::MAX - 1
        );
        assert_eq!(uniform_draw(u64::MAX, i64::MIN, i64::MAX - 1), i64::MIN);
        assert_eq!(uniform_draw(1_u64 << 63, -1, i64::MAX), i64::MAX);
        assert_eq!(uniform_draw((1_u64 << 63) + 1, -1, i64::MAX), -1);
        assert_eq!(uniform_draw(i64::MAX as u64, 0, i64::MAX), i64::MAX);
    }

    #[test]
    fn narrow_and_degenerate_ranges_keep_existing_results() {
        assert_eq!(uniform_draw(0, 9, 12), 9);
        assert_eq!(uniform_draw(3, 9, 12), 12);
        assert_eq!(uniform_draw(4, 9, 12), 9);
        assert_eq!(uniform_draw(3, -12, -9), -9);
        assert_eq!(uniform_draw(u64::MAX, i64::MAX - 1, i64::MAX), i64::MAX);
        assert_eq!(uniform_draw(5, 4, 4), 4);
        assert_eq!(uniform_draw(5, 4, -4), 4);
    }
}
