use super::*;
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, burn_fee_in_transaction,
};
use crate::durability::item_fee_burn_audit::OneItemTimedRepairV1;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::{OneItemStateV1, Type2Transaction};
use crate::durability::item_timed_repair::{
    RepairEntry, RepairFunds, RepairPlanError, RepairSnapshot, plan_soft_boots_repair,
};
use crate::durability::item_timed_state_audit::OneItemTimedRowV1;
use crate::durability::item_transfer::definition_message;

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &TimedRequest,
    coins: &FeeChangeFacts,
) -> Result<TimedItemResult> {
    let Some(target) = &request.target else {
        return Err(TimedWriteError::InvalidInput);
    };
    let Some(full) = target.full else {
        return Err(TimedWriteError::InvalidInput);
    };
    if request.definition.definition.production_key != "oteryn:item.tibia.i6530"
        || target.definition.production_key != "oteryn:item.tibia.i6529"
        || full.charges().is_some()
        || full.remaining_ms() != Some(14_400_000)
    {
        return Err(TimedWriteError::InvalidInput);
    }
    let bag: Option<String> = sqlx::query_scalar("SELECT i.item_instance_id::text FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let bag = uuid_text(&bag.ok_or(TimedWriteError::NotHeld)?)?;
    // At most the native backpack capacity plus one sentinel. Lock all direct item rows
    // in identity order, then let the native planner apply B3 placement order.
    let rows = sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,e.placement_ordinal::text, EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) AS contents FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid ORDER BY i.item_instance_id LIMIT 21 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(bag.as_slice()).fetch_all(&mut ***tx).await?;
    let mut entries = Vec::new();
    for row in rows {
        let item = uuid_text(&row.try_get::<String, _>("item_instance_id")?)?;
        let timed = timed::load_timed_state(tx, &item).await?;
        entries.push(RepairEntry {
            item_instance_id: item,
            parent: Some(bag),
            placement_ordinal: crate::durability::character_progression::numeric_u64(
                &row,
                "placement_ordinal",
            )?,
            definition: TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            live: row.try_get::<i16, _>("lifecycle")? == 1,
            has_contents: row.try_get("contents")?,
            timed,
        });
    }
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let balance: Option<i64> = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(uuid_text(&account)?.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let target_facts = TimedDefinitionFacts {
        definition: target.definition.clone(),
        full,
        lit_continuous: false,
    };
    let plan = match plan_soft_boots_repair(RepairSnapshot {
        backpack: bag,
        entries: &entries,
        from: &request.definition.definition,
        target: &target_facts,
        change: coins,
        funds: RepairFunds {
            tuple: tx.tuple().tuple(),
            junior: crate::durability::bank::fee_payer_is_junior(fence.character_id),
            bank_balance: balance.and_then(|n| u64::try_from(n).ok()).unwrap_or(0),
        },
    }) {
        Ok(plan) => plan,
        Err(RepairPlanError::InsufficientFunds) => {
            return Ok(TimedItemResult::refused(
                TimedDisposition::InsufficientFunds,
            ));
        }
        Err(_) => return Err(TimedWriteError::NotAStoreableChange),
    };
    if plan.item_instance_id != request.item {
        return Err(TimedWriteError::NotHeld);
    }
    let revision = plan.timed_before.map_or(0, |row| row.revision);
    if revision != request.expected_revision {
        return Ok(TimedItemResult::refused(TimedDisposition::Stale));
    }
    let state = |definition: &TypedDefinitionRef| OneItemStateV1 {
        item_instance_id: request.item.to_vec(),
        world_id: fence.runtime_scope.world_id().as_bytes().to_vec(),
        definition: Some(definition_message(definition)),
        quantity: 1,
        lifecycle: 1,
        key_number: 0,
    };
    let audit = OneItemTimedRepairV1 {
        occurrence: request.transaction_id.to_vec(),
        before: Some(state(&plan.before_definition)),
        after: Some(state(&plan.after_definition)),
        row_before: Some(OneItemTimedRowV1 {
            charges: None,
            remaining_ms: None,
            revision,
        }),
        row_after: Some(OneItemTimedRowV1 {
            charges: full.charges(),
            remaining_ms: full.remaining_ms(),
            revision: revision + 1,
        }),
        placement_ordinal: plan.placement_ordinal,
    };
    let fee = FeeBurnRequest {
        cause: FeeBurnCause::NpcRepair {
            occurrence: request.transaction_id,
            repair: Box::new(audit),
        },
        fee_gold_units: 10_000,
        transaction_id: request.transaction_id,
        event_id: request.event_id,
        occurred_at_unix_ms: request.occurred_at_unix_ms,
        server_build_id: "oteryn-game-server".into(),
        change: coins.clone(),
    };
    match burn_fee_in_transaction(tx, fence, &fee).await {
        Ok(_) => (),
        Err(FeeBurnError::InsufficientFunds) => {
            return Ok(TimedItemResult::refused(
                TimedDisposition::InsufficientFunds,
            ));
        }
        Err(FeeBurnError::Unavailable(error)) => return Err(error.into()),
        Err(_) => return Err(TimedWriteError::NotAStoreableChange),
    }
    sqlx::query("INSERT INTO game_item_timed_state_writes(item_instance_id,expected_revision,cause,transaction_id,holder_character_id,definition_before_family,definition_before_production_key,definition_before_revision_ref,definition_after_family,definition_after_production_key,definition_after_revision_ref,full_charges,full_remaining_ms,lit_continuous,charges_before,remaining_ms_before,deadline_before,charges_after,remaining_ms_after,deadline_after) VALUES(encode($1,'hex')::uuid,$2::text::numeric,8,encode($3,'hex')::uuid,encode($4,'hex')::uuid,'Item',$5,$6,'Item',$7,$8,NULL,14400000,false,NULL,NULL,NULL,NULL,14400000,NULL)")
        .bind(request.item.as_slice()).bind(revision.to_string()).bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&plan.before_definition.production_key).bind(&plan.before_definition.revision_ref).bind(&plan.after_definition.production_key).bind(&plan.after_definition.revision_ref).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,definition_revision_ref=$3,last_transaction_id=encode($4,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
        .bind(request.item.as_slice()).bind(&plan.after_definition.production_key).bind(&plan.after_definition.revision_ref).bind(request.transaction_id.as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,remaining_ms,deadline_at,state_revision) VALUES(encode($1,'hex')::uuid,NULL,14400000,NULL,1) ON CONFLICT(item_instance_id) DO UPDATE SET charges=NULL,remaining_ms=14400000,deadline_at=NULL,state_revision=game_item_timed_states.state_revision+1")
        .bind(request.item.as_slice()).execute(&mut ***tx).await?;
    Ok(TimedItemResult {
        disposition: TimedDisposition::Ok,
        revision: revision + 1,
        charges: None,
        remaining_ms: full.remaining_ms(),
        retired: false,
        fee_gold: 10_000,
    })
}
