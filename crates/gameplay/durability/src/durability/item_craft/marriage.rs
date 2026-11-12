//! NPC-MARRIAGE-1 Item participant. Existing instances, custody, Text and Craft audit.
use super::*;
use crate::durability::item_mint_audit::TransactionResourceUsage;
use crate::durability::npc_marriage::MarriageRequest;
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;

pub(crate) struct Plan {
    parent: Uuid,
    inputs: Vec<Value>,
    outputs: Vec<Value>,
    inscription: Option<String>,
    content_digest: [u8; 32],
}
/// Resolve the whole bounded plan before any stage/effect is inserted.
pub(crate) async fn prepare<C: ItemLoopContent>(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &MarriageRequest,
    operation: i16,
    occurred: i64,
    content: &C,
) -> Result<Result<Option<Plan>, &'static str>, DurabilityError> {
    if operation == 5 {
        return Ok(Ok(None));
    }
    let keys = [
        "oteryn:item.tibia.i3004",
        "oteryn:item.tibia.i9586",
        "oteryn:item.tibia.i9585",
    ];
    for (definition, key) in request.definitions.iter().zip(keys) {
        if definition.family != "Item"
            || definition.production_key != key
            || content.loop_facts(definition).is_none_or(|f| {
                f.weight.is_none() || stack_maximum(f.stack).ok() != Some((1, false))
            })
        {
            return Ok(Err("The native wedding items are unavailable."));
        }
    }
    let world = *fence.runtime_scope.world_id().as_bytes();
    let bag=sqlx::query("SELECT i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.world_id=s.world_id AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(bag) = bag else {
        return Ok(Err("Keep your wedding items in your main backpack."));
    };
    let parent: Uuid = bag.try_get("item_instance_id")?;
    let bag_definition = TypedDefinitionRef {
        family: bag.try_get("definition_family")?,
        production_key: bag.try_get("definition_production_key")?,
        revision_ref: bag.try_get("definition_revision_ref")?,
    };
    let capacity = content
        .loop_facts(&bag_definition)
        .and_then(|f| f.container_capacity)
        .unwrap_or(0)
        .min(20);
    let capacity = usize::try_from(capacity).map_err(|_| DurabilityError::InvalidStoredState)?;
    let rows=sqlx::query("SELECT to_jsonb(i) AS before,i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,e.placement_ordinal::text AS ordinal, NOT (EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_nested_entries c WHERE c.parent_item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)) AS plain FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=$2 AND e.world_id=encode($3,'hex')::uuid AND i.world_id=e.world_id ORDER BY i.item_instance_id LIMIT 21 FOR UPDATE OF i,e")
        .bind(fence.character_id.as_bytes().as_slice()).bind(parent).bind(world.as_slice()).fetch_all(&mut ***tx).await?;
    if rows.len() > capacity {
        return Ok(Err("There is no room in your backpack."));
    }
    let mut inputs = Vec::new();
    if operation == 1 || operation == 2 {
        for definition in &request.definitions[..2] {
            let input = rows.iter().find(|row| {
                row.try_get::<bool, _>("plain").unwrap_or(false)
                    && row.try_get::<i16, _>("lifecycle").ok() == Some(1)
                    && row.try_get::<i64, _>("quantity").ok() == Some(1)
                    && row
                        .try_get::<String, _>("definition_family")
                        .ok()
                        .as_deref()
                        == Some("Item")
                    && row
                        .try_get::<String, _>("definition_production_key")
                        .ok()
                        .as_deref()
                        == Some(&definition.production_key)
                    && row
                        .try_get::<String, _>("definition_revision_ref")
                        .ok()
                        .as_deref()
                        == Some(&definition.revision_ref)
            });
            let Some(input) = input else {
                return Ok(Err(
                    "You need a plain wedding ring and wedding outfit box in your main backpack.",
                ));
            };
            inputs.push(json!({"item":input.try_get::<Uuid,_>("item_instance_id")?.to_string(),"before":input.try_get::<Value,_>("before")?,"ordinal":input.try_get::<String,_>("ordinal")?}));
        }
    }
    let output_definitions = match operation {
        3 => &request.definitions[..2],
        4 => &request.definitions[2..],
        _ => &[],
    };
    if rows.len() + output_definitions.len() > capacity {
        return Ok(Err("Make room in your main backpack before continuing."));
    }
    if !output_definitions.is_empty() {
        let carried = match carried_weight(tx, fence.character_id, world, content).await {
            Ok(weight) => weight,
            Err(super::super::item_loop_move::LoopError::Durability(error)) => return Err(error),
            Err(super::super::item_loop_move::LoopError::Database(error)) => {
                return Err(error.into());
            }
            Err(super::super::item_loop_move::LoopError::Refused(_)) => {
                return Ok(Err("Your carried item weight is unavailable."));
            }
        };
        let capacity =
            read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
        let mut weight = carried;
        for definition in output_definitions {
            let Some(unit) = content.loop_facts(definition).and_then(|f| f.weight) else {
                return Ok(Err("The native wedding item weight is unavailable."));
            };
            weight = weight
                .checked_add(u64::from(unit))
                .ok_or(DurabilityError::InvalidStoredState)?;
        }
        if weight > capacity {
            return Ok(Err("You cannot carry the wedding items. Make room first."));
        }
    }
    let mut ordinal = 0_u64;
    for row in &rows {
        ordinal = ordinal.max(
            row.try_get::<String, _>("ordinal")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        );
    }
    let mut outputs = Vec::new();
    for definition in output_definitions {
        let Some(next) = ordinal.checked_add(1) else {
            return Ok(Err("No backpack output placement is available."));
        };
        ordinal = next;
        let item: Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut ***tx)
            .await?;
        outputs.push(json!({"item":item.to_string(),"key":definition.production_key,"revision":definition.revision_ref,"count":1,"ordinal":ordinal.to_string()}));
    }
    let inscription = if operation == 4 {
        Some(sqlx::query_scalar::<_,String>("SELECT a.name||' & '||b.name||' forever - married on '||to_char(to_timestamp($2::double precision/1000) AT TIME ZONE 'UTC','FMMonth DD, YYYY.') FROM game_character_roots a JOIN game_character_marriages m ON m.character_id=a.character_id JOIN game_character_roots b ON b.character_id=m.partner_id WHERE a.character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(occurred).fetch_one(&mut ***tx).await?)
    } else {
        None
    };
    let count = (inputs.len() + outputs.len()) as u64;
    TransactionResourceUsage {
        touched_item_instances: count,
        location_custody_lines: count,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: count,
        effect_work_units: if inscription.is_some() { 4 } else { count * 3 },
        events: 1,
    }
    .check()
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Ok(Some(Plan {
        parent,
        inputs,
        outputs,
        inscription,
        content_digest: content.loop_content_digest(),
    })))
}

pub(crate) async fn apply(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &MarriageRequest,
    plan: Plan,
    occurred: i64,
) -> Result<(), DurabilityError> {
    let transaction = Uuid::from_bytes(request.occurrence);
    let event = Uuid::from_bytes(request.event);
    let anchor = plan
        .inputs
        .first()
        .or_else(|| plan.outputs.first())
        .and_then(|v| v.get("item"))
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let inscription_hash = plan
        .inscription
        .as_ref()
        .map(|s| Sha256::digest(s.as_bytes()).to_vec());
    let envelope = audit::encode_map_harvest(
        fence,
        request.command,
        json!({"cause":"npc-marriage-v1","receipt":transaction.to_string(),"inputs":plan.inputs,"outputs":plan.outputs,"inscription_sha256":inscription_hash,"content_digest":plan.content_digest}),
        &transaction.to_string(),
        &event.to_string(),
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_marriage_records(transaction_id,event_id,character_id,world_id,parent_item_instance_id,inputs,outputs,anchor_item_id,occurred_at,inscription_sha256,envelope_sha256) VALUES($1,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10,sha256($11))")
        .bind(transaction).bind(event).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(plan.parent)
        .bind(json!(plan.inputs)).bind(json!(plan.outputs)).bind(anchor).bind(occurred).bind(&inscription_hash).bind(&envelope).execute(&mut ***tx).await?;
    for input in &plan.inputs {
        let item = input
            .get("item")
            .and_then(Value::as_str)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2 WHERE item_instance_id=$1::uuid AND quantity=1 AND lifecycle=1")
            .bind(item).bind(transaction).execute(&mut ***tx).await?.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
        if sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid AND character_id=encode($2,'hex')::uuid AND parent_item_instance_id=$3")
            .bind(item).bind(fence.character_id.as_bytes().as_slice()).bind(plan.parent).execute(&mut ***tx).await?.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
    }
    for output in &plan.outputs {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',1,1,$3)")
            .bind(output).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,($1->>'ordinal')::numeric,$5)")
            .bind(output).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(plan.parent).bind(transaction).execute(&mut ***tx).await?;
    }
    if let Some(text) = &plan.inscription {
        super::super::item_text::initialize_authored_reward_text(
            tx,
            *anchor.as_bytes(),
            request.occurrence,
            text,
            occurred,
        )
        .await?;
    }
    let tuple = tx.tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1,$2,1,1,2,$3,$4,$5,$6,$7,$8,sha256($8),1)")
        .bind(event).bind(transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(anchor).bind(occurred)
        .bind(occurred+super::super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(())
}
