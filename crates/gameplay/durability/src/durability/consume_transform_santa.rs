//! Exact Christmas gift USEs compose source fences with existing Type-2 Item mint.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::{
    NativeItemDestination, NativeItemStackClass,
};
use oteryn_gameplay_items::novelty_items::{GroundEffect, santa_gifts};

#[allow(clippy::too_many_arguments)]
pub(super) async fn write<C: ItemLoopContent>(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    permit: Option<&super::super::spell_owner_commit::SpellLanePermit>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    content: &C,
    before: &Value,
    source_kind: i32,
    source_parent: Option<&[u8]>,
    ordinal: &str,
    location: &Value,
    ground: Option<GroundEffect>,
) -> Result<ConsumeTransformResult, DurabilityError> {
    let Some(id) = before
        .get("definition_production_key")
        .and_then(Value::as_str)
        .and_then(|key| key.strip_prefix("oteryn:item.tibia.i"))
        .and_then(|id| id.parse::<u32>().ok())
        .filter(|id| santa_gifts::recognizes(*id))
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if !matches!(source_kind, 1 | 3 | 4 | 5)
        || request.target.is_some()
        || before.get("quantity").and_then(Value::as_i64) != Some(1)
        || before.get("lifecycle").and_then(Value::as_i64) != Some(1)
    {
        return Ok(ConsumeTransformResult::refused(
            UseDisposition::NothingToUse,
        ));
    }
    let Some(source_policy) = native.item_policy(
        before["definition_production_key"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?,
        before["definition_revision_ref"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?,
    ) else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    let source_carried = matches!(source_kind, 1 | 4)
        || (source_kind == 5
            && location
                .get("custody_kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind != "house"));
    let destination = if source_carried {
        NativeItemDestination::CharacterInventory
    } else {
        NativeItemDestination::Ground
    };
    if !source_policy.record().admission.materializable
        || source_policy.record().admission.stack_class != NativeItemStackClass::NonStackable
        || !source_policy
            .record()
            .admission
            .legal_destinations
            .contains(&destination)
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    }
    let Some(mut chosen) = santa_gifts::select(id, |domain, upper| {
        reward_bag_bounded_draw(
            *fence.game_session_id.as_bytes(),
            request.command.command_id().get(),
            request.source,
            domain,
            upper,
        )
    }) else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    // Speech originates at the clicked doll. The fifth dragon effect is on actor.
    chosen.effects.ground = ground;
    let mut effects =
        serde_json::to_value(&chosen.effects).map_err(|_| DurabilityError::InvalidStoredState)?;
    effects["gift_node"] = json!({"id":uuid_text(*node.fact().node_id().as_bytes()),"revision":node.fact().registration_revision().to_string()});
    let mut parent_capacity = 20;
    let mut outputs = Vec::new();
    let mut definitions = Vec::new();
    if let Some(coin) = chosen.currency {
        let bag=sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i ON i.item_instance_id=s.item_instance_id AND i.world_id=s.world_id WHERE s.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i,s")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
        let Some(bag) = bag else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
        };
        let parent: Vec<u8> = bag.try_get("item")?;
        let bag_definition = TypedDefinitionRef {
            family: bag.try_get("definition_family")?,
            production_key: bag.try_get("definition_production_key")?,
            revision_ref: bag.try_get("definition_revision_ref")?,
        };
        let Some(capacity) = content
            .loop_facts(&bag_definition)
            .and_then(|facts| facts.container_capacity)
        else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        parent_capacity = capacity;
        let (occupied,highest):(i64,String)=sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM (SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) entries")
            .bind(&parent).fetch_one(&mut ***tx).await?;
        let tree: i64 =
            sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice())
                .fetch_one(&mut ***tx)
                .await?;
        if occupied >= i64::from(capacity) || tree >= 500 {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
        }
        let Some(policy) = native.item_policy_for_source_id(coin) else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        if !policy.record().admission.materializable
            || !policy
                .record()
                .admission
                .legal_destinations
                .contains(&NativeItemDestination::CharacterInventory)
            || policy.record().production_definition.family != "Item"
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        let p = &policy.record().production_definition;
        let definition = TypedDefinitionRef {
            family: p.family.clone(),
            production_key: p.production_key.clone(),
            revision_ref: p.revision_ref.clone(),
        };
        let Some(coin_weight) = content
            .loop_facts(&definition)
            .and_then(|facts| facts.weight)
        else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        let carried = carried_weight(
            tx,
            fence.character_id,
            *fence.runtime_scope.world_id().as_bytes(),
            content,
        )
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut after_weight = carried
            .checked_add(u64::from(coin_weight))
            .ok_or(DurabilityError::InvalidStoredState)?;
        let carried_source = source_carried;
        if let Some(after_id) = chosen.transform {
            if before["state_revision"].as_u64() == Some(u64::MAX)
                || before["state_revision"].as_str() == Some("18446744073709551615")
            {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            let Some(after) = native.item_policy_for_source_id(after_id) else {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            };
            let destination = if carried_source {
                NativeItemDestination::CharacterInventory
            } else {
                NativeItemDestination::Ground
            };
            if !after.record().admission.materializable
                || after.record().admission.stack_class != NativeItemStackClass::NonStackable
                || !after
                    .record()
                    .admission
                    .legal_destinations
                    .contains(&destination)
            {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            let p = &after.record().production_definition;
            let target = TypedDefinitionRef {
                family: p.family.clone(),
                production_key: p.production_key.clone(),
                revision_ref: p.revision_ref.clone(),
            };
            if carried_source {
                let source = TypedDefinitionRef {
                    family: before["definition_family"]
                        .as_str()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .into(),
                    production_key: before["definition_production_key"]
                        .as_str()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .into(),
                    revision_ref: before["definition_revision_ref"]
                        .as_str()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .into(),
                };
                let (Some(old), Some(new)) = (
                    content.loop_facts(&source).and_then(|f| f.weight),
                    content.loop_facts(&target).and_then(|f| f.weight),
                ) else {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
                };
                after_weight = after_weight
                    .checked_sub(u64::from(old))
                    .and_then(|weight| weight.checked_add(u64::from(new)))
                    .ok_or(DurabilityError::InvalidStoredState)?;
            }
            effects["gift_after"] =
                json!({"key":target.production_key,"revision":target.revision_ref});
        }
        let limit =
            read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
        if after_weight > limit {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        let instance: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        let next = highest
            .parse::<u64>()
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(DurabilityError::InvalidStoredState)?;
        let parent =
            <[u8; 16]>::try_from(parent).map_err(|_| DurabilityError::InvalidStoredState)?;
        outputs.push(json!({"item":instance,"key":definition.production_key,"revision":definition.revision_ref,"count":1,"ordinal":next.to_string(),"decay":null,"gift_parent":uuid_text(parent)}));
        effects["gift_currency"] = json!(coin);
        definitions.push((definition, 1));
    }
    let plan = ActionPlan {
        recipe: format!("oteryn:novelty-item.i{id}.v1"),
        outputs: definitions,
        consume_source: false,
        source_count: 0,
        effects,
        extra_inputs: Vec::new(),
    };
    persist_action(
        permit,
        tx,
        fence,
        request,
        native,
        &plan,
        before,
        None,
        source_parent,
        source_kind,
        ordinal,
        parent_capacity,
        &outputs,
        location,
        None,
    )
    .await
}
