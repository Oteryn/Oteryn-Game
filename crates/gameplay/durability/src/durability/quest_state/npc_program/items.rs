//! Source NPC values compose the reused carried-input and once-MINT participants.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;

pub(super) fn ends(plan: &NpcRulePlan) -> bool {
    plan.effects
        .iter()
        .any(|effect| matches!(effect, NpcRuleEffect::EndConversation { .. }))
}

pub(super) fn end_delay(plan: &NpcRulePlan) -> u64 {
    plan.effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::EndConversation { delay_ms } => Some(*delay_ms),
            _ => None,
        })
        .min()
        .unwrap_or(0)
}

pub(super) fn changes_items(plan: &NpcRulePlan) -> bool {
    plan.effects.iter().any(|effect| {
        matches!(
            effect,
            NpcRuleEffect::TakeItem { .. } | NpcRuleEffect::GiveItem { .. }
        )
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "one source value participant shares the existing NPC fence and Type2 transaction"
)]
pub(super) async fn apply<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    content: &C,
    node: &NodeIncarnationProof,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
) -> StoredResult<bool> {
    if !crate::durability::quest_world_callback::consume_npc_inputs(
        tx,
        permit,
        fence,
        &request.quest_request(),
        plan,
    )
    .await?
    {
        return Ok(false);
    }
    let mut gifts = plan.effects.iter().filter_map(|effect| match effect {
        NpcRuleEffect::GiveItem {
            item,
            count,
            subtype: None,
        } => Some(serde_json::json!({"item": item, "count": count})),
        _ => None,
    });
    let gift = gifts.next().map(|mut primary| {
        let additional = gifts.collect::<Vec<_>>();
        if !additional.is_empty() {
            primary["additional"] = serde_json::Value::Array(additional);
        }
        primary
    });
    let minted = if gift.is_some() {
        match crate::durability::reward_claim_mint::npc_program::claim(
            tx,
            node,
            fence.item_fence(),
            request.command,
            content,
            plan,
        )
        .await?
        {
            Ok(transaction) => Some(transaction),
            Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => {
                return Ok(false);
            }
            Err(_) => return Err(DurabilityError::InvalidStoredState),
        }
    } else {
        None
    };
    let mut takes = BTreeMap::<String, u32>::new();
    for effect in &plan.effects {
        if let NpcRuleEffect::TakeItem { item, count, .. } = effect {
            let value = takes.entry(item.clone()).or_default();
            *value = value
                .checked_add(*count)
                .ok_or(DurabilityError::InvalidStoredState)?;
        }
    }
    let take = takes
        .into_iter()
        .map(|(item, count)| serde_json::json!({"item":item,"count":count}))
        .collect::<Vec<_>>();
    let mut value = serde_json::json!({"key":request.transition_key,"quest":plan.quest,"npc":request.npc,
        "program_digest":plan.program_digest,"rules":plan.selected_rule_keys,"take":take,"give":gift,
        "tracks":plan.track_changes.iter().map(|change|serde_json::json!({"key":change.key,"before":change.before,"after":change.after})).collect::<Vec<_>>()});
    if plan.completes {
        value["completes"] = serde_json::json!(true);
    }
    if value.to_string().len() > 16384 {
        return Ok(false);
    }
    sqlx::query("INSERT INTO game_quest_npc_item_sources(character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,CASE WHEN $6::bytea IS NULL THEN NULL ELSE encode($6,'hex')::uuid END)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(value)
        .bind(minted.map(|id|id.to_vec())).execute(&mut ***tx).await?;
    Ok(true)
}
