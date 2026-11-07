//! SRV7 closed NPC cosmetic and Cake substance costs in their caller's fenced transaction.
use super::DurabilityError;
use super::consume_transform::ConsumeTransformRequest;
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{self as audit, Type2Transaction};
use super::item_transfer::CurrentCharacterItemFence;
use super::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ClosedCostCause {
    NpcCosmetic,
    CakeSubstance,
    LyndaWedding,
}
impl ClosedCostCause {
    fn key(self) -> &'static str {
        match self {
            Self::NpcCosmetic => "npc_cosmetic",
            Self::CakeSubstance => "cake_substance",
            Self::LyndaWedding => "lynda_wedding",
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct NpcItemCost {
    pub definition: TypedDefinitionRef,
    pub count: u32,
}

struct Input {
    item: String,
    before: Value,
    location: Value,
    kind: String,
    count: u32,
}

/// The tree resolver already bounds depth to eight and backpack contents to 500.
/// Twelve equipment positions bring the physical read bound to 512.
async fn candidates(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    requirements: &Value,
) -> Result<Vec<Input>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind \
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
         JOIN game_item_instances i ON i.item_instance_id=c.item \
         WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' \
         AND c.kind IN ('container','nested','equipment') \
         AND EXISTS(SELECT 1 FROM jsonb_array_elements($3::jsonb) r WHERE \
           r->>'family'=i.definition_family AND r->>'key'=i.definition_production_key \
           AND r->>'revision'=i.definition_revision_ref) \
         AND NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) \
         AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) \
         ORDER BY i.item_instance_id LIMIT 513 FOR UPDATE OF i",
    ).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(requirements)
        .fetch_all(&mut ***tx).await?;
    if rows.len() > 512 {
        return Err(DurabilityError::InvalidStoredState);
    }
    rows.into_iter()
        .map(|r| {
            Ok(Input {
                item: r.try_get("item")?,
                before: r.try_get("before_item")?,
                location: r.try_get("location")?,
                kind: r.try_get("kind")?,
                count: 0,
            })
        })
        .collect()
}

fn definition(definition: &TypedDefinitionRef, count: u32) -> Value {
    json!({"family": definition.family, "key": definition.production_key,
        "revision": definition.revision_ref, "count": count})
}

/// The caller holds its live Character root/equipment fence in this same transaction.
pub(crate) async fn available(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    item: &TypedDefinitionRef,
) -> Result<u32, DurabilityError> {
    let requirements = json!([definition(item, 1)]);
    let mut total = 0u32;
    for input in candidates(tx, fence, &requirements).await? {
        let quantity = input.before["quantity"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        total = total
            .checked_add(quantity)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(total)
}

fn location_queries(
    permit: &SpellLanePermit,
    fence: CurrentCharacterItemFence,
    kind: &str,
) -> Result<(&'static str, &'static str), DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
    match kind {
        "container" => Ok((
            "SELECT to_jsonb(l) FROM game_item_container_entries l WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid",
        )),
        "nested" => Ok((
            "SELECT to_jsonb(l) FROM game_item_nested_entries l WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=$1::uuid",
        )),
        "equipment" => Ok((
            "SELECT to_jsonb(l) FROM game_character_equipment_slots l WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::uuid",
        )),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

/// `None` consumes nothing. The domain owner publishes the matching result receipt in the
/// same transaction; a deferred guard refuses a standalone or incomplete burn.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn burn(
    tx: &mut Type2Transaction<'_>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    cause: ClosedCostCause,
    source_key: &str,
    costs: &[NpcItemCost],
) -> Result<Option<String>, DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if command.game_session_id() != fence.game_session_id
        || costs.is_empty()
        || costs.len() > 16
        || source_key.is_empty()
        || source_key.len() > 256
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut required = BTreeMap::<(String, String, String), u32>::new();
    for cost in costs {
        if cost.count == 0 || cost.definition.family != "Item" {
            return Err(DurabilityError::InvalidStoredState);
        }
        let entry = required
            .entry((
                cost.definition.family.clone(),
                cost.definition.production_key.clone(),
                cost.definition.revision_ref.clone(),
            ))
            .or_default();
        *entry = entry
            .checked_add(cost.count)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let requirements = Value::Array(required.iter().map(|((family,key,revision), &count)|
        json!({"family":family,"key":key,"revision":revision,"count":count})).collect());
    let mut remaining = required.clone();
    let mut inputs = Vec::new();
    for mut input in candidates(tx, fence, &requirements).await? {
        let string = |field: &str| {
            input.before[field]
                .as_str()
                .map(str::to_owned)
                .ok_or(DurabilityError::InvalidStoredState)
        };
        let key = (
            string("definition_family")?,
            string("definition_production_key")?,
            string("definition_revision_ref")?,
        );
        let Some(left) = remaining.get_mut(&key) else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let quantity = input.before["quantity"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        input.count = quantity.min(*left);
        *left -= input.count;
        if input.count > 0 {
            inputs.push(input);
        }
    }
    if remaining.values().any(|&left| left > 0) {
        return Ok(None);
    }
    if inputs.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Resolve and lock every chosen custody row before writing any receipt or item.
    for input in &inputs {
        let (lock, _) = location_queries(permit, fence, &input.kind)?;
        let current: Option<Value> = sqlx::query_scalar(lock)
            .bind(&input.item)
            .fetch_optional(&mut ***tx)
            .await?;
        if current.as_ref() != Some(&input.location) {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let (transaction,event,occurred): (String,String,i64) = sqlx::query_as(
        "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut ***tx).await?;
    let input_proof = Value::Array(
        inputs
            .iter()
            .map(|i| {
                json!({"item":i.item,"before":i.before,
        "location":i.location,"kind":i.kind,"count":i.count})
            })
            .collect(),
    );
    let digest: Vec<u8> = sqlx::query_scalar("INSERT INTO game_srv7_item_costs(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,connection_generation,lease_generation,scope_generation,cause_kind,source_key,requirements,inputs,occurred_at,proof_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9::text::numeric,$10::text::numeric,$11,$12,$13,$14,$15,decode(repeat('00',32),'hex')) RETURNING proof_sha256")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(cause.key()).bind(source_key)
        .bind(&requirements).bind(&input_proof).bind(occurred).fetch_one(&mut ***tx).await?;
    for input in &inputs {
        let quantity = input.before["quantity"]
            .as_i64()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let updated = sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=$4")
            .bind(&input.item).bind(i64::from(input.count)).bind(&transaction).bind(quantity)
            .execute(&mut ***tx).await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        if quantity == i64::from(input.count) {
            let (_, delete) = location_queries(permit, fence, &input.kind)?;
            let deleted = sqlx::query(delete)
                .bind(&input.item)
                .execute(&mut ***tx)
                .await?;
            if deleted.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    if inputs.iter().any(|input| input.kind == "equipment") {
        let updated = sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$2::uuid WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&transaction)
            .execute(&mut ***tx).await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let first = inputs.first().ok_or(DurabilityError::InvalidStoredState)?;
    let source = super::item_mint::uuid_text(&first.item)?;
    let intent = serde_json::to_vec(&json!({"cause":cause.key(),"source":source_key}))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let request = ConsumeTransformRequest {
        command,
        source,
        target: None,
        intent,
    };
    let hex: String = digest.iter().map(|n| format!("{n:02x}")).collect();
    let proof = serde_json::to_vec(&json!({"schema":"OTERYN_CARRIED_COST_RECEIPT_COMMITMENT/v1",
        "encoding":"postgresql-jsonb-text-utf8/v1","transaction_id":transaction,"receipt_sha256":hex}))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let tuple = tx.tuple();
    let envelope = super::consume_transform_audit::encode(
        fence,
        &request,
        proof,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .bind(source.as_slice()).bind(occurred).bind(audit::AUDIT_RETENTION_P90D_MS).bind(&envelope)
        .execute(&mut ***tx).await?;
    Ok(Some(transaction))
}

/// A single bounded physical read for the finite source program's inventory predicates.
pub(crate) async fn available_many(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    items: &[(u32, TypedDefinitionRef)],
) -> Result<BTreeMap<u32, u32>, DurabilityError> {
    if items.len() > 128 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut ids = BTreeMap::new();
    for (id, item) in items {
        if ids
            .insert(
                (
                    item.family.clone(),
                    item.production_key.clone(),
                    item.revision_ref.clone(),
                ),
                *id,
            )
            .is_some()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let requirements = Value::Array(items.iter().map(|(_, item)| definition(item, 1)).collect());
    let mut result = BTreeMap::<u32, u32>::new();
    for row in candidates(tx, fence, &requirements).await? {
        let text = |key: &str| {
            row.before[key]
                .as_str()
                .map(str::to_owned)
                .ok_or(DurabilityError::InvalidStoredState)
        };
        let id = *ids
            .get(&(
                text("definition_family")?,
                text("definition_production_key")?,
                text("definition_revision_ref")?,
            ))
            .ok_or(DurabilityError::InvalidStoredState)?;
        let count = row.before["quantity"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        let current = result.entry(id).or_default();
        *current = current
            .checked_add(count)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(result)
}
