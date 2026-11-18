use std::collections::BTreeMap;

use super::{NpcRuleEffect, NpcRuleProgram};

// Donor storage declarations describe the Lua integer domain, not the values
// this programme writes. Only concrete writes can extend an existing owner range.
// Arithmetic and clock-derived writes remain checked against that range at plan time.
pub(super) fn literal_write_bounds(program: &NpcRuleProgram) -> BTreeMap<String, (i64, i64)> {
    let mut bounds = BTreeMap::new();
    let mut include = |track: &str, min: i64, max: i64| {
        let range = bounds.entry(track.to_owned()).or_insert((min, max));
        range.0 = range.0.min(min);
        range.1 = range.1.max(max);
    };
    for effect in program.document.rules.iter().flat_map(|rule| &rule.effects) {
        match effect {
            NpcRuleEffect::Set { track, value } => include(track, *value, *value),
            NpcRuleEffect::Draw { tracks, candidates } => {
                if let (Some(min), Some(max)) = (candidates.iter().min(), candidates.iter().max()) {
                    for track in tracks {
                        include(track, *min, *max);
                    }
                }
            }
            _ => {}
        }
    }
    bounds
}
