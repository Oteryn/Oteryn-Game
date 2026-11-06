//! Bounded carried ingredients reuse the quest consumption ledger and receipt.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::scope_of;
use oteryn_gameplay_quests::quest::{hot_cuisine, npc_rules::{NpcItemKey, NpcRuleEffect, NpcRuleGuard}};
use serde_json::Value;

pub(super) fn owns(source: &NpcRuleCatalog, request: &NpcProgramRequest) -> bool {
    request.npc == hot_cuisine::NPC || source.programs().iter()
        .flat_map(|program| &program.document.rules)
        .any(|rule| rule.npc == request.npc && rule.effects.iter().any(|effect|
            matches!(effect, NpcRuleEffect::TakeItem { .. } | NpcRuleEffect::GiveItem { .. })))
}

pub(super) fn optimistic(
    source: &NpcRuleCatalog, request: &NpcProgramRequest,
) -> std::result::Result<NpcInventorySnapshot, oteryn_gameplay_quests::quest::npc_rules::NpcRuleError> {
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleError;
    let mut inventory = NpcInventorySnapshot::default();
    let mut add = |item: &str, count: u32, subtype| -> std::result::Result<(), NpcRuleError> {
        let total = inventory.items.entry(NpcItemKey { item: item.into(), subtype }).or_default();
        *total = total.checked_add(count).ok_or(NpcRuleError::LimitExceeded)?;
        Ok(())
    };
    for rule in source.programs().iter().flat_map(|program| &program.document.rules).filter(|rule| rule.npc == request.npc) {
        for guard in &rule.guards {
            if let NpcRuleGuard::Item { item, count, subtype, .. } = guard { add(item, *count, *subtype)?; }
        }
        for effect in &rule.effects {
            if let NpcRuleEffect::TakeItem { item, count, subtype, .. } = effect { add(item, *count, *subtype)?; }
        }
    }
    if request.npc == hot_cuisine::NPC {
        for (key, count) in hot_cuisine::HotCuisineProgram::embedded()?.optimistic_inventory()?.items {
            add(&key.item, count, key.subtype)?;
        }
    }
    Ok(inventory)
}

struct Input {
    id: String,
    key: NpcItemKey,
    before: Value,
    location: Value,
    kind: String,
    quantity: u32,
    consumed: u32,
}

async fn inputs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, fence: CurrentCharacterGameplayFence,
) -> StoredResult<Vec<Input>> {
    let (world, _) = scope_of(&fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    // The Character root is already locked. Lock eligible instances in stable
    // identity order; restrict physical debits to ordinary carried bag entries.
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS id,i.definition_production_key,i.quantity,to_jsonb(i) AS before_item,c.location,c.kind, \
         (SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id) AS subtype \
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
         JOIN game_item_instances i ON i.item_instance_id=c.item \
         WHERE c.kind IN ('container','nested') AND i.lifecycle=1 AND i.definition_family='Item' \
         AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_imbuements t WHERE t.item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id) \
         ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 500 { return Err(DurabilityError::InvalidStoredState); }
    rows.into_iter().map(|row| Ok(Input {
        id: row.try_get("id")?,
        key: NpcItemKey {
            item: row.try_get("definition_production_key")?,
            subtype: row.try_get::<Option<i16>,_>("subtype")?.map(u16::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        before: row.try_get("before_item")?, location: row.try_get("location")?, kind: row.try_get("kind")?,
        quantity: u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?, consumed: 0,
    })).collect()
}

pub(super) async fn inventory(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, fence: CurrentCharacterGameplayFence,
) -> StoredResult<NpcInventorySnapshot> {
    let mut inventory = NpcInventorySnapshot::default();
    for input in inputs(tx, fence).await? {
        let total = inventory.items.entry(input.key).or_default();
        *total = total.checked_add(input.quantity).ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(inventory)
}

pub(super) async fn apply<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>, fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest, plan: &NpcRulePlan, content: &C, node: &NodeIncarnationProof,
) -> StoredResult<bool> {
    let mut inputs = inputs(tx, fence).await?;
    inputs.sort_by(|left, right| (&left.key, &left.id).cmp(&(&right.key, &right.id)));
    // Plan every debit before the first write. Repeated effects share remaining
    // quantities, and no stack or fluid container can satisfy two ingredients.
    for effect in &plan.effects {
        let NpcRuleEffect::TakeItem { item, count, subtype, .. } = effect else { continue };
        let mut remaining = *count;
        for input in &mut inputs {
            if input.key.item != *item || subtype.is_some_and(|value| input.key.subtype != Some(value)) { continue; }
            let count = remaining.min(input.quantity - input.consumed);
            input.consumed += count;
            remaining -= count;
            if remaining == 0 { break; }
        }
        if remaining != 0 { return Ok(false); }
    }
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut ***tx).await?;
    for input in inputs.iter().filter(|input| input.consumed > 0) {
        let proof = serde_json::to_vec(&serde_json::json!({
            "kind": "npc_quest_exchange", "binding": request.binding(fence.character_id).as_slice(),
            "program_digest": plan.program_digest, "effects": plan.effects,
            "before": input.before, "location": input.location, "subtype": input.key.subtype,
            "count": input.consumed,
        })).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,location_kind) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,NULL,$8,$9,sha256($9),$10,$11)")
            .bind(&transaction).bind(&input.id).bind(fence.character_id.as_bytes().as_slice())
            .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
            .bind(&request.transition_key).bind(&input.before).bind(i32::try_from(input.consumed).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(proof).bind(&input.location).bind(&input.kind).execute(&mut ***tx).await?;
        let changed = sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=$4")
            .bind(&input.id).bind(i64::from(input.consumed)).bind(&transaction).bind(i64::from(input.quantity)).execute(&mut ***tx).await?;
        if changed.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
        if input.quantity == input.consumed {
            let query = if input.kind == "container" { "DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid" }
                else { "DELETE FROM game_item_nested_entries WHERE item_instance_id=$1::uuid" };
            if sqlx::query(query).bind(&input.id).execute(&mut ***tx).await?.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    crate::durability::reward_claim_mint::npc_exchange::claim(tx, node, fence.item_fence(), request, plan, content).await
}
