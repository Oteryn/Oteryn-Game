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

/// Evaluate the same seeded decision used by the receipt owner.
pub fn draw_for_request(
    seed: [u8; 32],
    request: &NpcProgramRequest,
    character: CharacterId,
) -> Option<u32> {
    draw(seed, request.binding(character))
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
    if !valid_ticket_effects(&plan.effects) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let draw =
        draw_for_request(seed, request, character).ok_or(DurabilityError::InvalidStoredState)?;
    let outcome = SANDRA_LOTTERY
        .draw(draw)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut resolved = plan.clone();
    resolved.inventory_after = SANDRA_LOTTERY
        .inventory_after(&plan.inventory_before, draw)
        .ok_or(DurabilityError::InvalidStoredState)?;
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

/// Each ticket command owns one MINT claim; separate tickets remain repeatable.
/// The existing command receipt still handles replay before any RNG or minting.
pub fn claim_key(session: [u8; 16], command: u64) -> String {
    let session: String = session.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("oteryn:reward.sandra-lottery.{session}.{command}")
}

/// Reserve room for a possible prize before drawing or consuming the ticket.
/// The Character root lock held by the NPC writer serializes inventory writers.
pub(super) async fn has_prize_room<C: crate::durability::item_loop_move::ItemLoopContent>(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    character: CharacterId,
    content: &C,
) -> StoredResult<bool> {
    use crate::durability::item_transfer::{load_entries, load_slot};
    let Some(slot) = load_slot(tx, character).await? else {
        return Ok(false);
    };
    let Some(capacity) = content
        .loop_facts(&slot.definition)
        .and_then(|facts| facts.container_capacity)
    else {
        return Ok(false);
    };
    let entries = load_entries(tx, slot.item_instance_id).await?;
    Ok(u32::try_from(entries.len()).is_ok_and(|count| count < capacity))
}

/// Reject extra or optional inventory effects before choosing a random reward.
pub fn valid_ticket_effects(effects: &[NpcRuleEffect]) -> bool {
    let mut takes = 0;
    let mut gives = 0;
    for effect in effects {
        match effect {
            NpcRuleEffect::TakeItem {
                item,
                count: 1,
                subtype: None,
                optional: false,
            } if item == SANDRA_LOTTERY.input => takes += 1,
            NpcRuleEffect::GiveItem {
                item,
                count: 1,
                subtype: None,
            } if item == "oteryn:item.tibia.i5958" => gives += 1,
            NpcRuleEffect::Set { track, value: 1 }
                if track
                    == "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt" => {}
            NpcRuleEffect::SetTopic { value: 0 } => {}
            _ => return false,
        }
    }
    takes == 1 && gives == 1
}
