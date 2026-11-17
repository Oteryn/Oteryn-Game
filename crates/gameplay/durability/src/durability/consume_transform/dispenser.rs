//! Crystal cask_and_kegs.lua:33-54, in the existing receipt/audit transaction.
use super::super::item_timed_state::{
    TimedCheckpointRequest, TimedDefinitionFacts, write_checkpoint,
};
use super::*;
use crate::domain::timed_item::TimedValues;
use oteryn_gameplay_content::content::ReferenceItemField;
use oteryn_gameplay_items::remaining_consumables::keg_fill;

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    flask: &TypedDefinitionRef,
    quantity: i64,
    native: &NativeGameplayState,
    context: ActionContext<'_>,
    node: &NodeIncarnationProof,
) -> Result<Option<ActionPlan>, DurabilityError> {
    let Some(target) = request.target.filter(|target| *target != request.source) else {
        return Ok(None);
    };
    let Some(flask_id) = source_id(&flask.production_key).filter(|id| matches!(id, 283..=285))
    else {
        return Ok(None);
    };
    if flask.family != "Item"
        || !native
            .item_policy(&flask.production_key, &flask.revision_ref)
            .is_some_and(|policy| policy.record().admission.materializable)
    {
        return Ok(None);
    }
    let placed = match context {
        ActionContext::DispenserTarget(ground) => ground.dispenser_row(tx, fence, target).await?,
        ActionContext::DispenserHouseTarget(house) => house.dispenser_row(tx, node, target).await?,
        _ => None,
    };
    let row = if placed.is_some() || matches!(context, ActionContext::DispenserTarget(_)) {
        placed
    } else {
        sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,uuid_send(coalesce(e.parent_item_instance_id,n.parent_item_instance_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN n.item_instance_id IS NULL THEN 1 ELSE 4 END AS source_kind FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR (n.world_id=i.world_id AND owner.owner_character=encode($2,'hex')::uuid AND owner.depth BETWEEN 0 AND 7)) FOR UPDATE OF i")
        .bind(target.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut ***tx).await?
    };
    let Some(row) = row else { return Ok(None) };
    let key: String = row.try_get("definition_production_key")?;
    if row.try_get::<String, _>("definition_family")? != "Item" {
        return Ok(None);
    }
    let revision: String = row.try_get("definition_revision_ref")?;
    let Some(id) = source_id(&key) else {
        return Ok(None);
    };
    let Some(policy) = native
        .item_policy(&key, &revision)
        .filter(|policy| policy.record().admission.materializable)
    else {
        return Ok(None);
    };
    let ReferenceItemField::Known(charges) = &policy.record().semantics.charges else {
        return Ok(None);
    };
    let ReferenceItemField::Known(initial) = charges.count else {
        return Ok(None);
    };
    let initial = u16::try_from(initial).map_err(|_| DurabilityError::InvalidStoredState)?;
    let stored = sqlx::query("SELECT charges,remaining_ms,deadline_at,state_revision::text FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(target.as_slice()).fetch_optional(&mut ***tx).await?;
    let (before, expected_revision) = match stored {
        None => (initial, 0),
        Some(row) => {
            if row.try_get::<Option<i64>, _>("remaining_ms")?.is_some()
                || row.try_get::<Option<i64>, _>("deadline_at")?.is_some()
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let charges: i32 = row.try_get("charges")?;
            let revision: String = row.try_get("state_revision")?;
            (
                u16::try_from(charges).map_err(|_| DurabilityError::InvalidStoredState)?,
                revision
                    .parse::<u64>()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
        }
    };
    let count = u16::try_from(quantity).map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(fill) = keg_fill(id, flask_id, count, before) else {
        return Ok(None);
    };
    let output_key = format!("oteryn:item.tibia.i{}", fill.potion_id);
    let Some(output) = native
        .item_policy(&output_key, "definition-r1")
        .filter(|p| p.record().admission.materializable)
    else {
        return Ok(None);
    };
    let definition = &output.record().production_definition;
    let extra_inputs = if fill.removes_keg() {
        vec![ExtraInput {
            item: target,
            before: row.try_get("before_item")?,
            parent: row
                .try_get::<Vec<u8>, _>("parent")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ordinal: row.try_get("ordinal")?,
            count: 1,
            source_kind: row.try_get("source_kind")?,
        }]
    } else {
        Vec::new()
    };
    Ok(Some(ActionPlan {
        recipe: "oteryn:dispenser.fill".into(),
        outputs: vec![(
            TypedDefinitionRef {
                family: definition.family.clone(),
                production_key: definition.production_key.clone(),
                revision_ref: definition.revision_ref.clone(),
            },
            u32::from(fill.filled),
        )],
        consume_source: true,
        source_count: u32::from(fill.filled),
        extra_inputs,
        effects: json!({"kind":"dispenser", "dispenser": {
            "item":uuid_text(target), "initial":initial, "before":before, "after":fill.charges_remaining,
            "before_item":row.try_get::<Value, _>("before_item")?, "filled":fill.filled,
            "expected_revision":expected_revision,
            "source_kind":row.try_get::<i32,_>("source_kind")?,
            "location":if matches!(row.try_get::<i32,_>("source_kind")?,3|5) { row.try_get::<Value,_>("ground_location")? } else { Value::Null },
            "house_context":match context { ActionContext::DispenserHouseTarget(house)=>house.dispenser_proof(node), _=>Value::Null },
        }}),
    }))
}

pub(super) async fn apply(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    plan: &ActionPlan,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let Some(dispenser) = plan.effects.get("dispenser") else {
        return Ok(());
    };
    let after = dispenser["after"]
        .as_u64()
        .ok_or(DurabilityError::InvalidStoredState)?;
    // As in the timed expiry owner, depletion retires the item and retains its
    // preceding timed row. A live row never stores zero charges.
    if after == 0 {
        return Ok(());
    }
    let before = &dispenser["before_item"];
    let text = |key: &str| {
        before[key]
            .as_str()
            .map(str::to_owned)
            .ok_or(DurabilityError::InvalidStoredState)
    };
    let initial = dispenser["initial"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let values = |charges| {
        TimedValues::new(Some(charges), None).map_err(|_| DurabilityError::InvalidStoredState)
    };
    let item: sqlx::types::Uuid = dispenser["item"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let transaction = super::super::item_mint::uuid_text(transaction)?;
    write_checkpoint(
        tx,
        fence.character_id,
        &TimedCheckpointRequest {
            item_instance_id: *item.as_bytes(),
            expected_revision: dispenser["expected_revision"]
                .as_u64()
                .ok_or(DurabilityError::InvalidStoredState)?,
            transaction_id: transaction,
            definition: TimedDefinitionFacts {
                definition: TypedDefinitionRef {
                    family: text("definition_family")?,
                    production_key: text("definition_production_key")?,
                    revision_ref: text("definition_revision_ref")?,
                },
                full: values(initial)?,
                lit_continuous: false,
            },
            values: values(u32::try_from(after).map_err(|_| DurabilityError::InvalidStoredState)?)?,
        },
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

fn source_id(key: &str) -> Option<u32> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}
