//! BED0 §7.2: offline bed credits, independent of vocation and online tick phase.

/// Apply bounded sleep to durable pools and food milliseconds. Online fractional
/// periods ride through unchanged; food is charged even when both pools are full.
#[must_use]
pub fn settle(
    pools: [u32; 3],
    maxima: [u32; 3],
    food_ms: u32,
    sleep_s: u64,
) -> Option<([u32; 3], u32)> {
    if food_ms > 1_200_000 || maxima[0] == 0 || pools[0] == 0 {
        return None;
    }
    let sleep_s = sleep_s.min(1_814_400);
    let points = u32::try_from((u64::from(food_ms / 1000).min(sleep_s)) / 30).ok()?;
    let soul = u32::try_from(sleep_s / 900).ok()?;
    Some((
        [
            pools[0].saturating_add(points).min(maxima[0]),
            pools[1].saturating_add(points).min(maxima[1]),
            pools[2].saturating_add(soul).min(maxima[2]),
        ],
        food_ms.checked_sub(points.checked_mul(30_000)?)?,
    ))
}

#[cfg(test)]
#[path = "house_bed_regen_tests.rs"]
mod tests;
