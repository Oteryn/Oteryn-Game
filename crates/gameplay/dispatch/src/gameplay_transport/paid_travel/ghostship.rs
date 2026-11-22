use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

use super::TravelOccurrence;

// Crystal 201189f8, data-global/npc/captain_fearless.lua:85–98.
pub(super) const WARNING: &str =
    "I warn you! This route is haunted by a ghostship. Do you really want to go there?";
pub(super) const DECLINE: &str = "We would like to serve you some time.";

pub(super) fn diverts_to_ghostship(seed: [u8; 32], occurrence: TravelOccurrence) -> Option<bool> {
    const OUTCOMES: u64 = 10;
    let root = GameplayDecisionRoot::from_bytes(seed);
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence.as_bytes());
    // Exclude the incomplete final group to give each of the ten results equal weight.
    for index in 0..=u64::MAX {
        let draw =
            deterministic_decision_u64(&root, occurrence, "captain-fearless-darashia", index)
                .ok()?;
        if draw < u64::MAX - u64::MAX % OUTCOMES {
            return Some(draw.is_multiple_of(OUTCOMES));
        }
    }
    None
}
