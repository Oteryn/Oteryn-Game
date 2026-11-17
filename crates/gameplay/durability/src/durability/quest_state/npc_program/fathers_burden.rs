//! Tereban's fixed eight materials and final cape use the existing item audit
//! and quest-XP obligation owners in the source NPC receipt transaction.
use super::*;
use crate::durability::item_craft::{ItemCraftRequest, audit};
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::scope_of;
use oteryn_gameplay_quests::quest::npc_rules::{NpcItemKey, NpcRuleEffect};
use serde_json::Value;

const MATERIALS: [u32; 8] = [11545, 11546, 11547, 11548, 11549, 11550, 11551, 11552];
const CAPE: &str = "oteryn:item.tibia.i11701";
const BURN: &str = "oteryn:npc-handoff.father-material";

pub(super) fn owns(request: &NpcProgramRequest) -> bool {
    request.npc == "oteryn:npc.tereban"
        && request
            .transition_key
            .starts_with("oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/")
        && (request.transition_key.ends_with("-deliver")
            || request.transition_key.ends_with("/mission-complete"))
}

// Preview can propose a branch, never authorize consumption. The transaction
// reevaluates that same source programme with the locked carried inventory.
pub(super) fn optimistic_inventory() -> NpcInventorySnapshot {
    NpcInventorySnapshot {
        items: MATERIALS
            .into_iter()
            .map(|number| {
                (
                    NpcItemKey {
                        item: format!("oteryn:item.tibia.i{number}"),
                        subtype: None,
                    },
                    1,
                )
            })
            .collect(),
        ..NpcInventorySnapshot::default()
    }
}

pub(super) async fn inventory(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> StoredResult<NpcInventorySnapshot> {
    let keys = MATERIALS.map(|number| format!("oteryn:item.tibia.i{number}"));
    inventory_for(tx, fence, &keys).await
}

pub(super) async fn inventory_for(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    keys: &[String],
) -> StoredResult<NpcInventorySnapshot> {
    let (world, _) =
        scope_of(&fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let rows = sqlx::query("SELECT i.definition_production_key,i.quantity,(SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id) AS fluid_subtype FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3) ORDER BY i.item_instance_id LIMIT 501")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(keys).fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut inventory = NpcInventorySnapshot::default();
    for row in rows {
        let count = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let key = NpcItemKey {
            item: row.try_get("definition_production_key")?,
            subtype: row.try_get::<Option<i16>, _>("fluid_subtype")?.map(u16::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        let total = inventory.items.entry(key).or_default();
        *total = total
            .checked_add(count)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(inventory)
}

pub(super) fn experience(plan: &NpcRulePlan) -> StoredResult<i64> {
    let amounts = plan
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::Experience { amount } => Some(*amount),
            _ => None,
        })
        .collect::<Vec<_>>();
    match amounts.as_slice() {
        [2500] => Ok(2500),
        [8000] => Ok(8000),
        _ => Err(DurabilityError::InvalidStoredState),
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
    let takes = plan
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::TakeItem {
                item,
                count: 1,
                subtype: None,
                optional: false,
            } => Some(item.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let gives = plan
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::GiveItem {
                item,
                count: 1,
                subtype: None,
            } => Some(item.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if request.transition_key.ends_with("/mission-complete") {
        if !takes.is_empty() || gives != [CAPE] || experience(plan)? != 8000 {
            return Err(DurabilityError::InvalidStoredState);
        }
        return match crate::durability::reward_claim_mint::fathers_burden::claim(
            tx,
            node,
            fence.item_fence(),
            request.command,
            content,
        )
        .await?
        {
            Ok(()) => Ok(true),
            Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => {
                Ok(false)
            }
            Err(_) => Err(DurabilityError::InvalidStoredState),
        };
    }
    if takes.len() != 1
        || !gives.is_empty()
        || experience(plan)? != 2500
        || !MATERIALS
            .iter()
            .any(|number| takes[0] == format!("oteryn:item.tibia.i{number}"))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    burn(tx, fence, request, content, takes[0], BURN).await
}

pub(super) async fn burn<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    content: &C,
    item: &str,
    recipe: &str,
) -> StoredResult<bool> {
    let recovery_digest = content.loop_content_digest();
    let (world, _) =
        scope_of(&fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let source = sqlx::query("SELECT to_jsonb(i) AS before,uuid_send(i.item_instance_id) AS source,uuid_send(b.item_instance_id) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN e.item_instance_id IS NULL THEN to_jsonb(n) ELSE NULL END AS nested_source FROM game_item_container_slots b JOIN game_item_instances i ON i.world_id=b.world_id LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=b.character_id AND e.parent_item_instance_id=b.item_instance_id AND e.world_id=b.world_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id AND n.world_id=b.world_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE b.character_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 AND (e.item_instance_id IS NOT NULL OR (owner.owner_character=b.character_id AND owner.depth BETWEEN 1 AND 7)) ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(item).fetch_optional(&mut ***tx).await?;
    let Some(source) = source else {
        return Ok(false);
    };
    let before: Value = source.try_get("before")?;
    let source_id: [u8; 16] = source
        .try_get::<Vec<u8>, _>("source")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let parent: Vec<u8> = source.try_get("parent")?;
    let ordinal: String = source.try_get("ordinal")?;
    let nested: Option<Value> = source.try_get("nested_source")?;
    let (transaction,event,occurred):(String,String,i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let craft = ItemCraftRequest {
        command: request.command,
        source: source_id,
        tool: None,
        ingredient: None,
        station: None,
        intent: request.binding(fence.character_id).to_vec(),
        gathering: None,
        enchanting: None,
    };
    let envelope = audit::encode(
        fence.item_fence(),
        &craft,
        recipe,
        recovery_digest,
        &before,
        &[],
        &transaction,
        &event,
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,event_source_location) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,'[]'::jsonb,$13,sha256($14),$15)")
        .bind(&transaction).bind(&event).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&craft.intent).bind(recipe).bind(source_id.as_slice()).bind(&parent).bind(&ordinal).bind(&before).bind(occurred).bind(&envelope).bind(&nested).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid").bind(source_id.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
    if before["quantity"] == 1 {
        let delete = if nested.is_some() {
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        } else {
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        };
        sqlx::query(delete)
            .bind(source_id.as_slice())
            .execute(&mut ***tx)
            .await?;
    }
    outbox(tx, &event, &transaction, &source_id, occurred, &envelope).await?;
    Ok(true)
}

async fn outbox(
    tx: &mut Type2Transaction<'_>,
    event: &str,
    transaction: &str,
    item: &[u8],
    occurred: i64,
    envelope: &[u8],
) -> StoredResult<()> {
    let tuple = tx.tuple().tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)").bind(event).bind(transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(item).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut ***tx).await?;
    Ok(())
}
