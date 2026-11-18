use std::collections::BTreeMap;

use super::{NpcRuleEffect, NpcRuleProgram};

// Donor storage declarations describe the Lua integer domain, not the values
// this programme writes. Only concrete writes can extend an existing owner range.
// Arithmetic and clock-derived writes remain checked against that range at plan time.
pub(super) fn literal_write_bounds(program: &NpcRuleProgram) -> BTreeMap<String, (i64, i64)> {
    let mut bounds = BTreeMap::new();
    for effect in program.document.rules.iter().flat_map(|rule| &rule.effects) {
        let (track, min, max) = match effect {
            NpcRuleEffect::Set { track, value } => (track, *value, *value),
            NpcRuleEffect::SetRandom {
                track,
                minimum,
                maximum,
            } => (track, i64::from(*minimum), i64::from(*maximum)),
            _ => continue,
        };
        let range = bounds.entry(track.clone()).or_insert((min, max));
        range.0 = range.0.min(min);
        range.1 = range.1.max(max);
    }
    bounds
}
