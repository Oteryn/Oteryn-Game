//! Pemaret's fixed fish debit and once-only trophy share the locked NPC receipt transaction.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_quests::quest::npc_rules::{NpcItemKey, NpcRuleEffect};

pub(super) const FISH: &str = "oteryn:item.tibia.i901";
const TROPHY: &str = "oteryn:item.tibia.i902";
const TRACK: &str = "oteryn:quest-progress/quest/u7_8/marlin_trophy";

pub(super) fn owns(request: &NpcProgramRequest) -> bool {
    request.npc == "oteryn:npc.pemaret"
}

pub(super) fn optimistic_inventory() -> NpcInventorySnapshot {
    NpcInventorySnapshot {
        items: BTreeMap::from([(
            NpcItemKey {
                item: FISH.into(),
                subtype: None,
            },
            1,
        )]),
        ..NpcInventorySnapshot::default()
    }
}

pub(super) async fn apply<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    content: &C,
    node: &NodeIncarnationProof,
) -> StoredResult<bool> {
    if request.transition_key != "oteryn:quest-npc/marlin_trophy_quest/pemaret/deliver"
        || plan.track_changes.len() != 1
        || plan.track_changes[0].key != TRACK
        || !(-1..=0).contains(&plan.track_changes[0].before)
        || plan.track_changes[0].after != 1
        || plan.effects
            != [
                NpcRuleEffect::TakeItem {
                    item: FISH.into(),
                    count: 1,
                    subtype: None,
                    optional: false,
                },
                NpcRuleEffect::GiveItem {
                    item: TROPHY.into(),
                    count: 1,
                    subtype: None,
                },
                NpcRuleEffect::Set {
                    track: TRACK.into(),
                    value: 1,
                },
                NpcRuleEffect::SetTopic { value: 0 },
            ]
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !fathers_burden::burn(
        tx,
        fence,
        request,
        content,
        FISH,
        "oteryn:npc-handoff.marlin",
    )
    .await?
    {
        return Ok(false);
    }
    match crate::durability::reward_claim_mint::marlin_trophy::claim(
        tx,
        node,
        fence.item_fence(),
        request.command,
        content,
    )
    .await?
    {
        Ok(()) => Ok(true),
        Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => Ok(false),
        Err(_) => Err(DurabilityError::InvalidStoredState),
    }
}
