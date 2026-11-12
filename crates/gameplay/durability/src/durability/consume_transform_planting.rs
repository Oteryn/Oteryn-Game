//! Exact seed use on materialized pots, within the existing action/custody owner.
use super::*;
use oteryn_gameplay_items::novelty_items::GroundEffect;

pub(super) async fn prepare_ground(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    pot: &TypedDefinitionRef,
    native: &NativeGameplayState,
    position: GroundEffect,
) -> Result<Option<ActionPlan>, DurabilityError> {
    prepare(tx, fence, request, pot, native, Some(position)).await
}

pub(super) async fn prepare_carried(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    pot: &TypedDefinitionRef,
    native: &NativeGameplayState,
) -> Result<Option<ActionPlan>, DurabilityError> {
    prepare(tx, fence, request, pot, native, None).await
}

async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    pot: &TypedDefinitionRef,
    native: &NativeGameplayState,
    position: Option<GroundEffect>,
) -> Result<Option<ActionPlan>, DurabilityError> {
    let Some(seed) = request.target else {
        return Ok(None);
    };
    if !matches!(
        pot.production_key.as_str(),
        "oteryn:item.tibia.i306" | "oteryn:item.tibia.i14029"
    ) || request.source == seed
        || pot.family != "Item"
        || native
            .item_policy(&pot.production_key, &pot.revision_ref)
            .is_none()
    {
        return Ok(None);
    }
    let row = sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,uuid_send(coalesce(e.parent_item_instance_id,n.parent_item_instance_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN n.item_instance_id IS NULL THEN 1 ELSE 4 END AS source_kind FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR (n.world_id=i.world_id AND owner.owner_character=encode($2,'hex')::uuid AND owner.depth BETWEEN 0 AND 7)) FOR UPDATE OF i")
        .bind(seed.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let seeds = TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    };
    if seeds.family != "Item"
        || seeds.production_key != "oteryn:item.tibia.i13844"
        || native
            .item_policy(&seeds.production_key, &seeds.revision_ref)
            .is_none()
    {
        return Ok(None);
    }
    let preserve = pot.production_key == "oteryn:item.tibia.i14029";
    let (recipe, outputs) = if preserve {
        ("oteryn:planting.preserve".to_owned(), Vec::new())
    } else {
        let Some(conversion) =
            oteryn_gameplay_items::event_planting::seed_conversion(native, &seeds, pot)
        else {
            return Ok(None);
        };
        (conversion.recipe, conversion.outputs)
    };
    Ok(Some(ActionPlan {
        recipe,
        outputs,
        consume_source: !preserve,
        source_count: u32::from(!preserve),
        effects: if preserve {
            oteryn_gameplay_items::event_planting::preservation_effects(position)
        } else {
            oteryn_gameplay_items::event_planting::seed_effects(position)
        },
        extra_inputs: vec![ExtraInput {
            item: seed,
            before: row.try_get("before_item")?,
            parent: row
                .try_get::<Vec<u8>, _>("parent")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ordinal: row.try_get("ordinal")?,
            count: 1,
            source_kind: row.try_get("source_kind")?,
        }],
    }))
}
