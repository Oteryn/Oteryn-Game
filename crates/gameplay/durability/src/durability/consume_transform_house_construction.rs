//! Native opaque-handle USE delegates construction to the existing House placement owner.
use super::*;
type ConstructionReplayRow = (Option<Vec<u8>>, Vec<u8>, Vec<u8>, String);
use oteryn_gameplay_content::content::ReferenceItemField as F;
use oteryn_protocol_oteryn::house_wrap as wire;

pub(super) const SOURCE: &str = "SELECT * FROM game_house_construction_source(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid)";

pub(super) async fn candidate(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    context: &HouseActionContext,
    request: &ConsumeTransformRequest,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i WHERE (EXISTS(SELECT 1 FROM game_house_furniture_rules c WHERE c.action=2 AND i.definition_production_key='oteryn:item.tibia.i'||c.first_item::text) OR (i.definition_production_key='oteryn:item.tibia.i23398' AND (EXISTS(SELECT 1 FROM game_house_furniture_kit_origins k WHERE k.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=i.item_instance_id AND r.action=1 AND r.after_key=i.definition_production_key)))) AND i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id AND b.world_id=e.world_id AND b.character_id=e.character_id WHERE e.item_instance_id=i.item_instance_id AND e.world_id=i.world_id AND e.character_id=encode($3,'hex')::uuid))")
        .bind(request.source.as_slice()).bind(context.house.world_id().as_bytes().as_slice())
        .bind(context.fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut ***tx).await?)
}

pub(super) async fn replay(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    context: &HouseActionContext,
    request: &ConsumeTransformRequest,
) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
    let previous:Option<ConstructionReplayRow>=sqlx::query_as("SELECT use_intent,uuid_send(character_id),uuid_send(world_id),house_key FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(context.fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .fetch_optional(&mut ***tx).await?;
    Ok(previous.map(|(intent, character, world, house)| {
        if intent.as_deref() == Some(request.intent.as_slice())
            && character == context.fence.character_id.as_bytes()
            && world == context.house.world_id().as_bytes()
            && house == context.house.house_key()
        {
            ConsumeTransformResult {
                disposition: UseDisposition::Committed,
                fresh: false,
                effects: json!({}),
                recipe: Some("oteryn:house-construction.v1".into()),
            }
        } else {
            ConsumeTransformResult::refused(UseDisposition::Rejected)
        }
    }))
}

pub(super) async fn apply(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    context: &HouseActionContext,
    request: &ConsumeTransformRequest,
    before: &Value,
    native: &NativeGameplayState,
) -> Result<ConsumeTransformResult, DurabilityError> {
    let definition = before
        .get("definition_production_key")
        .and_then(Value::as_str)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let target = if let Some(id) = oteryn_gameplay_items::furniture::construction_to(definition) {
        Some(format!("oteryn:item.tibia.i{id}"))
    } else if definition == "oteryn:item.tibia.i23398" {
        sqlx::query_scalar("SELECT target_key FROM (SELECT r.before_item->>'definition_production_key' AS target_key,1 AS priority,(r.before_item->>'state_revision')::numeric AS ordering FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.action=1 AND r.after_key='oteryn:item.tibia.i23398' UNION ALL SELECT target_key,2,0 FROM game_house_furniture_kit_origins WHERE item_instance_id=encode($1,'hex')::uuid) origins ORDER BY priority,ordering DESC LIMIT 1")
            .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?
    } else {
        None
    };
    let Some(target) = target else {
        return Ok(ConsumeTransformResult::refused(
            UseDisposition::NothingToUse,
        ));
    };
    let Some(policy) = native.item_policy(definition, "definition-r1") else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    let Some(output) = native
        .item_policy(&target, "definition-r1")
        .map(|policy| policy.record())
    else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    if !policy.record().admission.materializable || !output.admission.materializable {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    }
    let acl:String=sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).fetch_one(&mut ***tx).await?;
    let backpack:Option<String>=sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'")
        .bind(context.fence.character_id.as_bytes().as_slice()).bind(context.house.world_id().as_bytes().as_slice())
        .fetch_optional(&mut ***tx).await?;
    let capacity = backpack
        .as_ref()
        .and_then(|key| native.item_policy(key, "definition-r1"))
        .and_then(|p| match &p.record().semantics.container {
            F::Known(container) => match container.capacity {
                F::Known(capacity) => Some(i32::from(capacity)),
                _ => None,
            },
            _ => None,
        });
    let input = wire::Request {
        action: wire::Action::Unwrap,
        item: request.source,
        expected_revision: before
            .get("state_revision")
            .and_then(Value::as_u64)
            .ok_or(DurabilityError::InvalidStoredState)?,
        acl_revision: acl
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        position: context.position,
    };
    let row=fenced("SELECT game_house_furniture_construct(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,$10::text::numeric,encode($11,'hex')::uuid,$12::text::numeric,$13::text::numeric,$14::smallint,$15,$16,$17,$18,$19,$20)::text",node,context)
        .bind(request.command.command_id().get().to_string()).bind(request.source.to_vec())
        .bind(input.expected_revision.to_string()).bind(input.acl_revision.to_string()).bind(2_i16)
        .bind(context.position.iter().flat_map(|n|n.to_be_bytes()).collect::<Vec<_>>()).bind(input.encode())
        .bind(&output.production_definition.production_key).bind(capacity).bind(backpack).bind(&request.intent)
        .fetch_one(&mut ***tx).await;
    match row {
        Ok(_) => {}
        Err(error) => {
            return match error.as_database_error().map(|e| e.message()) {
                Some("PODIUM_STALE_SESSION" | "WRAP_STALE") => {
                    Ok(ConsumeTransformResult::refused(UseDisposition::StaleState))
                }
                Some("WRAP_NO_ACCESS" | "WRAP_NOT_WRAPABLE") => {
                    Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible))
                }
                Some("WRAP_BLOCKED") => Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom)),
                Some("WRAP_INVENTORY_FULL") => {
                    Ok(ConsumeTransformResult::refused(UseDisposition::Full))
                }
                _ => Err(error.into()),
            };
        }
    }
    let transaction:String=sqlx::query_scalar("SELECT transaction_id::text FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(context.fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .fetch_one(&mut ***tx).await?;
    sqlx::query("SELECT game_house_furniture_timer_apply($1::uuid)")
        .bind(transaction)
        .execute(&mut ***tx)
        .await?;
    Ok(ConsumeTransformResult {
        disposition: UseDisposition::Committed,
        fresh: true,
        effects: json!({}),
        recipe: Some("oteryn:house-construction.v1".into()),
    })
}
