//! Receipt-bound source NPC experience, donation and numbered-key rewards.
use super::*;
use crate::domain::currency::Coin;
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, FeeChangeFacts,
    burn_quest_reward_fee_in_transaction,
};
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::Type2Transaction;

use oteryn_gameplay_quests::quest::{QUESTGATE0_RL_05, npc_rules::NpcRuleEffect};

pub(super) fn has_experience(effects: &[NpcRuleEffect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, NpcRuleEffect::Experience { .. }))
}

pub(super) fn experience(effects: &[NpcRuleEffect]) -> StoredResult<i64> {
    effects.iter().try_fold(0_i64, |total, effect| {
        let NpcRuleEffect::Experience { amount } = effect else {
            return Ok(total);
        };
        let amount = i64::try_from(*amount).map_err(|_| DurabilityError::InvalidStoredState)?;
        let total = total
            .checked_add(amount)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if amount <= 0 || total > QUESTGATE0_RL_05 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(total)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_xp_is_summed_once_within_the_shared_obligation_bound()
    -> std::result::Result<(), DurabilityError> {
        assert_eq!(experience(&[])?, 0);
        assert_eq!(
            experience(&[
                NpcRuleEffect::Experience { amount: 7 },
                NpcRuleEffect::Experience { amount: 11 }
            ])?,
            18
        );
        assert_eq!(
            experience(&[NpcRuleEffect::Experience {
                amount: 100_000_000
            }])?,
            QUESTGATE0_RL_05
        );
        for amounts in [[0, 1], [100_000_000, 1], [u64::MAX, 1]] {
            assert!(
                experience(&amounts.map(|amount| NpcRuleEffect::Experience { amount })).is_err()
            );
        }
        Ok(())
    }
}

pub(super) const SKJAAR_DONATION: &str = "oteryn:quest-reward/skjaar-donation";

pub(super) enum SkjaarPayment {
    Paid,
    InsufficientFunds,
    ChangeDoesNotFit,
}

pub(super) fn skjaar_fee(plan: &NpcRulePlan) -> StoredResult<Option<u64>> {
    use oteryn_gameplay_quests::quest::npc_rules::SKJAAR_QUEST;
    let mut fees = plan
        .effects
        .iter()
        .filter(|effect| matches!(effect, NpcRuleEffect::TakeMoney { .. }));
    match (fees.next(), fees.next()) {
        (None, None) => Ok(None),
        (
            Some(NpcRuleEffect::TakeMoney {
                amount,
                include_bank: true,
            }),
            None,
        ) if plan.quest == SKJAAR_QUEST
            && plan
                .selected_rule_keys
                .first()
                .is_some_and(|key| key == SKJAAR_DONATION) =>
        {
            Ok(Some(*amount))
        }
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

pub(super) fn skjaar_refusal(reply: Vec<String>, next_topic: i32) -> NpcProgramCommit {
    NpcProgramCommit {
        reply,
        next_topic,
        read_only: true,
        ..refused(QuestRefusal::StageMismatch)
    }
}

pub(super) async fn debit_skjaar_fee(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    content: &impl ItemLoopContent,
    fee: u64,
) -> StoredResult<SkjaarPayment> {
    let change = fee_change(tx, fence, content).await?;
    let row = sqlx::query(
        "SELECT game_character_uuid_v7()::text AS occurrence,
                game_character_uuid_v7()::text AS transaction_id,
                game_character_uuid_v7()::text AS event_id,
                floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at",
    )
    .fetch_one(&mut ***tx)
    .await?;
    let occurrence = uuid_text(row.try_get("occurrence")?)?;
    let burn = FeeBurnRequest {
        cause: FeeBurnCause::QuestRewardPayment {
            callback: request.transition_key.clone(),
            occurrence,
            gold: fee,
            include_bank: true,
        },
        fee_gold_units: fee,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        server_build_id: "oteryn-game-server".into(),
        change,
    };
    let tuple = tx.tuple();
    let committed = match burn_quest_reward_fee_in_transaction(tx, &fence, &burn, tuple).await {
        Ok(FeeBurnOutcome::Burned(receipt)) => receipt.committed_character_revision,
        Err(FeeBurnError::InsufficientFunds) => return Ok(SkjaarPayment::InsufficientFunds),
        Err(FeeBurnError::CapacityExceeded | FeeBurnError::ChangeDoesNotFit) => {
            return Ok(SkjaarPayment::ChangeDoesNotFit);
        }
        Err(FeeBurnError::Unavailable(error)) => return Err(error),
        _ => return Err(DurabilityError::InvalidStoredState),
    };

    let character = fence.character_id.as_bytes().as_slice();
    let session = request.command.game_session_id();
    let command = request.command.command_id().get().to_string();
    let gold = i64::try_from(fee).map_err(|_| DurabilityError::InvalidStoredState)?;
    let source = serde_json::json!({
        "key": request.transition_key,
        "quest": plan.quest,
        "npc": request.npc,
        "storage": [],
        "take": [],
        "give": null,
        "outfit": null,
        "fee": {"gold": gold, "include_bank": true},
        "npc_response": {"reply": plan.reply, "next_topic": plan.next_topic},
    });
    sqlx::query(
        "INSERT INTO game_quest_reward_sources
             (character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id,grant_keys)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,NULL,ARRAY[]::text[])",
    ).bind(character).bind(session.as_bytes().as_slice()).bind(&command)
      .bind(&request.transition_key).bind(source).execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO game_quest_reward_fees
             (character_id,fee_occurrence,cause_id,cause_ordinal,transition_key,fee_gold_units,include_bank,committed_character_revision)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,true,$7::text::numeric)",
    ).bind(character).bind(occurrence.as_slice()).bind(session.as_bytes().as_slice())
      .bind(command).bind(&request.transition_key).bind(gold).bind(committed.get().to_string())
      .execute(&mut ***tx).await?;
    Ok(SkjaarPayment::Paid)
}

async fn fee_change(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    content: &impl ItemLoopContent,
) -> StoredResult<FeeChangeFacts> {
    let index = content
        .npc_outfit_content()
        .and_then(|native| native.item_index())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let definition = |key: &str| -> StoredResult<TypedDefinitionRef> {
        let revision = index
            .revision(key)
            .ok_or(DurabilityError::InvalidStoredState)?;
        Ok(TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        })
    };
    let backpack = match crate::durability::item_transfer::load_slot(tx, fence.character_id).await?
    {
        Some(slot) => slot.definition,
        None => definition("oteryn:item.tibia.i2854")?,
    };
    let backpack = content
        .loop_facts(&backpack)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (platinum_id, gold_id): (String, String) =
        sqlx::query_as("SELECT game_character_uuid_v7()::text, game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
    Ok(FeeChangeFacts {
        item_instance_ids: [uuid_text(&platinum_id)?, uuid_text(&gold_id)?],
        gold: definition(Coin::Gold.production_key())?,
        platinum: definition(Coin::Platinum.production_key())?,
        crystal: definition(Coin::Crystal.production_key())?,
        backpack,
    })
}

/// The fee's existing source proof has a quest-reward key, while the ordinary
/// NPC response table only admits quest-npc keys. Replay the response retained
/// in that source row after the same NPC request binding has been checked.
pub(super) async fn replay_skjaar_donation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
    transition: QuestTransitionOutcome,
) -> StoredResult<NpcProgramCommit> {
    #[derive(serde::Deserialize)]
    struct Response {
        reply: Vec<String>,
        next_topic: i32,
    }

    let response: sqlx::types::Json<Response> = sqlx::query_scalar(
        "SELECT s.plan->'npc_response'
         FROM game_quest_reward_sources s JOIN game_quest_reward_fees f
           USING(character_id,cause_id,cause_ordinal,transition_key)
         WHERE s.character_id=encode($1,'hex')::uuid AND s.cause_id=encode($2,'hex')::uuid
           AND s.cause_ordinal=$3::text::numeric AND s.transition_key=$4",
    )
    .bind(character.as_bytes().as_slice())
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(SKJAAR_DONATION)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(NpcProgramCommit {
        transition,
        reply: response.0.reply,
        next_topic: response.0.next_topic,
        read_only: false,
        inventory_changed: true,
        end_conversation: false,
        end_delay_ms: 0,
        magic_effects: Vec::new(),
    })
}

pub(super) async fn mint_skjaar_key(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    content: &impl ItemLoopContent,
) -> StoredResult<bool> {
    use crate::durability::reward_claim_mint::{RewardClaimMintError, npc_program};
    let mut gifts = plan
        .effects
        .iter()
        .filter(|effect| matches!(effect, NpcRuleEffect::GiveItem { .. }));
    let (
        Some(NpcRuleEffect::GiveItem {
            item,
            count: 1,
            subtype: Some(key_number),
        }),
        None,
    ) = (gifts.next(), gifts.next())
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let transaction =
        match npc_program::claim(tx, node, fence.item_fence(), request.command, content, plan)
            .await?
        {
            Ok(transaction) => transaction,
            Err(RewardClaimMintError::Refused(_)) => return Ok(false),
            Err(_) => return Err(DurabilityError::InvalidStoredState),
        };
    let tracks = plan.track_changes.iter().map(|change| {
        serde_json::json!({"key": change.key, "before": change.before, "after": change.after})
    }).collect::<Vec<_>>();
    let source = serde_json::json!({
        "key": request.transition_key,
        "quest": plan.quest,
        "npc": request.npc,
        "program_digest": plan.program_digest,
        "rules": plan.selected_rule_keys,
        "tracks": tracks,
        "take": [],
        "give": {"item": item, "count": 1, "key_number": key_number},
    });
    sqlx::query(
        "INSERT INTO game_quest_npc_item_sources
             (character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,encode($6,'hex')::uuid)",
    ).bind(fence.character_id.as_bytes().as_slice())
      .bind(request.command.game_session_id().as_bytes().as_slice())
      .bind(request.command.command_id().get().to_string())
      .bind(&request.transition_key).bind(source).bind(transaction.as_slice())
      .execute(&mut ***tx).await?;
    Ok(true)
}
