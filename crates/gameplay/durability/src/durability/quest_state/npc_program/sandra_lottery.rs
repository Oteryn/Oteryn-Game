//! Receipt-bound lottery draws from the shared server decision RNG owner.
use super::*;
use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleEffect, random_outcome::SANDRA_LOTTERY};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

/// The generation digest is the production seed; tests supply an explicit seed.
/// Binding to the fenced character and command keeps retries on the same draw.
pub fn draw(seed: [u8; 32], binding: [u8; 33]) -> Option<u32> {
    let root = GameplayDecisionRoot::from_bytes(seed);
    let digest = Sha256::digest(binding);
    let mut occurrence = [0_u8; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let upper = u64::from(SANDRA_LOTTERY.total_weight()?);
    let limit = u64::MAX - u64::MAX % upper;
    for index in 0..1024 {
        let value = deterministic_decision_u64(&root, occurrence, "sandra.lottery", index).ok()?;
        if value < limit {
            return u32::try_from(value % upper).ok();
        }
    }
    None
}

pub(super) fn resolve(
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    seed: [u8; 32],
    character: CharacterId,
) -> StoredResult<Option<NpcRulePlan>> {
    if request.npc != "oteryn:npc.sandra"
        || request.transition_key != "oteryn:quest-npc/sandra/lottery"
    {
        return Ok(None);
    }
    // Only the declared ticket exchange may enter this random value participant.
    if !plan.effects.iter().any(|effect| {
        matches!(effect,
        NpcRuleEffect::TakeItem { item, count: 1, subtype: None, optional: false }
        if item == SANDRA_LOTTERY.input)
    }) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let outcome = draw(seed, request.binding(character))
        .and_then(|value| SANDRA_LOTTERY.draw(value))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut resolved = plan.clone();
    resolved
        .effects
        .retain(|effect| !matches!(effect, NpcRuleEffect::GiveItem { .. }));
    if let Some(item) = outcome {
        resolved.effects.push(NpcRuleEffect::GiveItem {
            item: item.into(),
            count: 1,
            subtype: None,
        });
    }
    Ok(Some(resolved))
}
