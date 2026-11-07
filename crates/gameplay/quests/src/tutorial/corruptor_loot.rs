//! Source vocation rewards. Missing definitions are handled as an entire absent reward.
pub fn corruptor_items(vocation: &str) -> &'static [u32] {
    match super::steps::vocation_hint(vocation) {
        Some(3) => &[3359, 3285, 3375],
        Some(4) => &[10321, 48843],
        Some(5) => &[3074, 8871],
        Some(6) => &[3066, 8871],
        Some(11) => &[48847, 48845],
        _ => &[],
    }
}
