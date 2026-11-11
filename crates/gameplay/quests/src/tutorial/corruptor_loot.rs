//! Current Newhaven gear identities (Wiki Corruptor revid 1200696).
//! Crystal reward names select the vocation; its older server IDs are not client IDs.
pub fn corruptor_items(vocation: &str) -> &'static [u32] {
    match super::steps::vocation_hint(vocation) {
        Some(3) => &[3359, 3285, 3375],
        Some(4) => &[3571, 51760],
        Some(5) => &[3074, 7991],
        Some(6) => &[3066, 7991],
        Some(11) => &[50195, 51119],
        _ => &[],
    }
}
