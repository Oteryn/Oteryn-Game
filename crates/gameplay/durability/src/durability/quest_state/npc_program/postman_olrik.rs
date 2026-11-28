//! Resolve the paid Olrik roll inside its receipt transaction, after replay lookup.
use super::{
    CurrentCharacterGameplayFence, DurabilityError, NpcInventorySnapshot, NpcProgramRequest,
    NpcRulePlan, StoredResult, uuid_text,
};
use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin};
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts, burn_quest_reward_fee_in_transaction,
};
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::Type2Transaction;
use sqlx::PgConnection;
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;

pub(super) const NPC: &str = "oteryn:npc.olrik";
pub(super) const WAGER: u64 = 5;
pub(super) const ROLL: &str = "oteryn:quest-npc/postman/olrik/roll";
const CREDIT: &str = "oteryn:quest-progress/quest/u7_24/the_postman_missions/measurements_olrik";

pub(super) fn resolve(plan: &mut NpcRulePlan, face: i16) -> StoredResult<()> {
    let [win, loss] = plan.reply.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let template = match face {
        6 => win,
        1..=5 => loss,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    plan.reply = vec![template.replace("{roll}", &face.to_string())];

    if face != 6 {
        // The wager is charged on either result. A losing roll records an
        // unchanged credit so the receipt can replay the paid result exactly.
        plan.track_changes.retain(|change| change.key == CREDIT);
        let [credit] = plan.track_changes.as_mut_slice() else {
            return Err(DurabilityError::InvalidStoredState);
        };
        credit.after = credit.before;
        plan.effects
            .retain(|effect| matches!(effect, NpcRuleEffect::TakeMoney { .. }));
        plan.next_topic = 1;
    }
    Ok(())
}

/// Advisory totals use the same carried-coin universe as the authoritative fee owner.
/// The fee writer holds the Type2 activation fence while it rereads these totals.
pub(super) async fn inventory(
    tx: &mut PgConnection,
    fence: CurrentCharacterGameplayFence,
) -> StoredResult<NpcInventorySnapshot> {
    let (money, bank): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT coalesce(sum(i.quantity * game_item_coin_worth(i.definition_production_key)),0)::bigint
                    FROM game_atelier_carried_coin_sources(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
                    JOIN game_item_instances i USING(item_instance_id)),
                (SELECT coalesce(max(b.balance),0)::bigint
                    FROM game_account_bank_balances b
                    JOIN game_character_roots r ON r.account_id=b.account_id AND r.world_id=b.world_id
                    WHERE r.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid
                      AND EXISTS (SELECT 1 FROM game_type2_audit_activation WHERE id=1))",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await?;
    Ok(NpcInventorySnapshot {
        money: u64::try_from(money).map_err(|_| DurabilityError::InvalidStoredState)?,
        bank: if crate::durability::bank::fee_payer_is_junior(fence.character_id) {
            0
        } else {
            u64::try_from(bank).map_err(|_| DurabilityError::InvalidStoredState)?
        },
        ..NpcInventorySnapshot::default()
    })
}

pub(super) async fn charge<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    content: &C,
) -> StoredResult<bool> {
    if !plan.effects.iter().any(|effect| {
        matches!(effect, NpcRuleEffect::TakeMoney { amount: WAGER, include_bank: true })
    }) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let index = content
        .npc_outfit_content()
        .and_then(|native| native.item_index())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let definition = |coin: Coin| {
        let key = coin.production_key();
        Ok::<_, DurabilityError>(TypedDefinitionRef {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: key.into(),
            revision_ref: index.revision(key).ok_or(DurabilityError::InvalidStoredState)?.into(),
        })
    };
    let character = fence.character_id.as_bytes().as_slice();
    let backpack: Option<(String, String)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.definition_revision_ref
           FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id)
          WHERE b.character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_optional(&mut ***tx)
    .await?;
    let backpack = match backpack {
        Some((production_key, revision_ref)) => TypedDefinitionRef {
            family: "Item".into(), production_key, revision_ref,
        },
        None => {
            let key = "oteryn:item.tibia.i2854";
            TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: index.revision(key).ok_or(DurabilityError::InvalidStoredState)?.into(),
            }
        }
    };
    let Some(backpack) = content.loop_facts(&backpack) else {
        return Ok(false);
    };
    let (ids, now): (Vec<String>, i64) = sqlx::query_as(
        "SELECT ARRAY(SELECT game_character_uuid_v7()::text FROM generate_series(1,5)),
                floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
    )
    .fetch_one(&mut ***tx)
    .await?;
    let [occurrence, transaction_id, event_id, platinum_change, gold_change] = ids.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let fee = FeeBurnRequest {
        cause: FeeBurnCause::QuestRewardPayment {
            callback: ROLL.into(),
            occurrence: uuid_text(occurrence)?,
            gold: WAGER,
            include_bank: true,
        },
        fee_gold_units: WAGER,
        transaction_id: uuid_text(transaction_id)?,
        event_id: uuid_text(event_id)?,
        occurred_at_unix_ms: now,
        server_build_id: "oteryn-game-server".into(),
        change: FeeChangeFacts {
            item_instance_ids: [uuid_text(platinum_change)?, uuid_text(gold_change)?],
            gold: definition(Coin::Gold)?,
            platinum: definition(Coin::Platinum)?,
            crystal: definition(Coin::Crystal)?,
            backpack,
        },
    };
    let tuple = tx.tuple();
    let committed = match burn_quest_reward_fee_in_transaction(tx, &fence, &fee, tuple).await {
        Ok(crate::durability::item_fee_burn::FeeBurnOutcome::Burned(receipt)) => receipt,
        Err(FeeBurnError::InsufficientFunds | FeeBurnError::CapacityExceeded | FeeBurnError::ChangeDoesNotFit) => return Ok(false),
        Err(FeeBurnError::Unavailable(error)) => return Err(error),
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let session = request.command.game_session_id().as_bytes().to_vec();
    let ordinal = request.command.command_id().get().to_string();
    let source = serde_json::json!({
        "key": ROLL, "quest": plan.quest, "take": [], "give": null, "outfit": null,
        "fee": { "gold": WAGER, "include_bank": true },
    });
    sqlx::query(
        "INSERT INTO game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key,plan,grant_keys)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,ARRAY[]::text[])",
    )
    .bind(character).bind(&session).bind(&ordinal).bind(ROLL).bind(source)
    .execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO game_quest_reward_fees(character_id,fee_occurrence,cause_id,cause_ordinal,transition_key,fee_gold_units,include_bank,committed_character_revision)
         VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,true,$7::text::numeric)",
    )
    .bind(character).bind(occurrence).bind(&session).bind(&ordinal).bind(ROLL)
    .bind(i64::try_from(WAGER).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(committed.committed_character_revision.get().to_string())
    .execute(&mut ***tx).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;

    #[test]
    fn paid_olrik_roll_only_awards_credit_on_six() -> Result<(), String> {
        let catalog = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
        let mission = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
        let tracks = BTreeMap::from([(mission.to_owned(), 5)]);
        let inventory = NpcInventorySnapshot {
            money: 2,
            bank: 3,
            ..Default::default()
        };
        let source = catalog
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.olrik",
                trigger: NpcRuleTrigger::Say,
                message: "yes",
                player_name: None,
                topic: 1,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "",
                sex: 0,
                outfits: &BTreeMap::new(),
            })
            .map_err(|error| error.to_string())?
            .ok_or("paid Olrik roll")?;
        assert_eq!(source.selected_rule_keys, [ROLL]);
        for face in 1..=6 {
            let mut plan = source.clone();
            resolve(&mut plan, face).map_err(|error| format!("{error:?}"))?;
            assert_eq!(plan.inventory_after.money, 0);
            assert_eq!(plan.inventory_after.bank, 0);
            assert!(plan.effects.iter().any(|effect| matches!(
                effect, NpcRuleEffect::TakeMoney { amount: WAGER, include_bank: true }
            )));
            assert_eq!(plan.next_topic, i32::from(face != 6));
            assert_eq!(
                plan.track_changes
                    .iter()
                    .find(|change| change.key == mission)
                    .map(|change| change.after),
                (face == 6).then_some(6),
            );
            assert_eq!(
                plan.track_changes
                    .iter()
                    .find(|change| change.key == CREDIT)
                    .map(|change| change.after),
                Some(i64::from(face == 6)),
            );
            let template = &source.reply[usize::from(face != 6)];
            assert_eq!(plan.reply, [template.replace("{roll}", &face.to_string())]);
        }
        Ok(())
    }
}
