//! Source vocation rewards. Missing definitions are handled as an entire absent reward.
pub fn corruptor_items(vocation: &str) -> &'static [u32] {
    match super::steps::vocation_hint(vocation) {
        Some(3) => &[3359, 3285, 3375],
        // Q3/Q10: missing and contradictory donor IDs keep the entire
        // vocation reward unavailable, even when a conflicting ID exists.
        _ => &[],
    }
}
