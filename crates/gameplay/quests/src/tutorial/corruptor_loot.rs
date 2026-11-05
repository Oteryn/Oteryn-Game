//! Named Newhaven rewards, reconciled against Crystal 201189f's exact XML identities.
//! The disabled donor callback's stale IDs are recorded in quest-finish-newhaven-items.json.
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
