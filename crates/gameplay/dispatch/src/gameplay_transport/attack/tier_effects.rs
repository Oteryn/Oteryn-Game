//! Global Exaltation Forge proc chances from currently equipped items.

use super::cyclopedia::amplification_ppm;
use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_simulation_determinism::deterministic_decision_u64;

/// TibiaWiki Ruse rates, in hundredths of a percent, indexed by armor tier.
const RUSE: [u64; 11] = [0, 50, 103, 162, 228, 300, 378, 462, 552, 648, 751];

fn tier_proc(
    rates: &[u64; 11],
    tier: u8,
    boots: u8,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    purpose: &str,
) -> bool {
    let base = rates.get(usize::from(tier)).copied().unwrap_or(0);
    if base == 0 {
        return false;
    }
    let multiplier = 1_000_000 + amplification_ppm(boots) as u64;
    let threshold = u128::from(base * multiplier) << 64;
    deterministic_decision_u64(root, occurrence, purpose, 0)
        .is_ok_and(|draw| u128::from(draw) * 10_000_000_000 < threshold)
}

pub(super) fn ruse(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    target: ExactActorRef,
    session: GameSessionId,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> bool {
    states.get(runtime, target, session).is_some_and(|state| {
        let equipment = state.equipped_offence();
        tier_proc(
            &RUSE,
            equipment.armor_tier,
            equipment.boots_tier,
            root,
            occurrence,
            "tier_ruse",
        )
    })
}
