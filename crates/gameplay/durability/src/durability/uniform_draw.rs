/// Uniform draw in `[minimum, maximum]` from one deterministic decision value. Canary and Crystal
/// draw with `normal_random`; the Reference distribution is not evidenced, so this stays a
/// replaceable world rule.
pub fn uniform_draw(decision: u64, minimum: i64, maximum: i64) -> i64 {
    if maximum <= minimum {
        return minimum;
    }
    let span = u64::try_from(maximum - minimum)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let offset = i64::try_from(decision % span).unwrap_or(0);
    minimum + offset
}
