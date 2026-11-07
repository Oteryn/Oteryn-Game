//! Carried Rotate preserves the same instance, every child and physical custody.
use super::super::consume_transform_cosmetics::Preparation;
use super::*;
use oteryn_gameplay_content::content::native_gameplay::{
    NativeItemDestination, NativeItemStackClass,
};
use oteryn_protocol_oteryn::world_object::{UseTarget, decode_use_intent_target};
use std::num::NonZeroU32;

pub(super) async fn prepare_carried(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    source_kind: i32,
    node: &NodeIncarnationProof,
) -> Result<Preparation, DurabilityError> {
    let rotate = request.intent.first() == Some(&0xff);
    let portable =
        oteryn_gameplay_items::furniture::portable_use_to(&definition.production_key).is_some();
    if !rotate && !portable {
        return Ok(Preparation::Unrelated);
    }
    let refused = |d| Ok(Preparation::Refused(d));
    if !matches!(source_kind, 1 | 3 | 4) || request.target.is_some() {
        return refused(UseDisposition::NothingToUse);
    }
    let wire = if rotate {
        if request.intent.len() < 6 {
            return refused(UseDisposition::Rejected);
        }
        let slot = u32::from_be_bytes(
            request.intent[1..5]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        );
        let Some(expected) =
            NonZeroU32::new(slot).and_then(|slot| native.item_index()?.definition_at(slot))
        else {
            return refused(UseDisposition::StaleState);
        };
        if expected
            != (
                definition.production_key.as_str(),
                definition.revision_ref.as_str(),
            )
        {
            return refused(UseDisposition::StaleState);
        }
        &request.intent[5..]
    } else {
        request.intent.as_slice()
    };
    let Ok(intent) = decode_use_intent_target(&[4, 11, 15], wire) else {
        return refused(UseDisposition::Rejected);
    };
    if !matches!(intent.target, UseTarget::Item(_))
        || intent.use_with.is_some()
        || intent.use_with_map_item.is_some()
        || intent.use_with_position.is_some()
    {
        return refused(UseDisposition::NothingToUse);
    }
    let selected = if rotate {
        oteryn_gameplay_items::furniture::plan_rotate(native, definition)
    } else {
        oteryn_gameplay_items::furniture::plan_portable_use(native, definition)
    };
    let Some(mut selected) = selected else {
        return refused(UseDisposition::NothingToUse);
    };
    let [(after, 1)] = selected.outputs.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    for candidate in [definition, after] {
        let Some(policy) = native.item_policy(&candidate.production_key, &candidate.revision_ref)
        else {
            return refused(UseDisposition::NotPossible);
        };
        let record = policy.record();
        if record.production_definition.family != candidate.family
            || !record.admission.materializable
            || record.admission.stack_class != NativeItemStackClass::NonStackable
            || !record
                .admission
                .legal_destinations
                .contains(&if source_kind == 3 {
                    NativeItemDestination::Ground
                } else {
                    NativeItemDestination::CharacterInventory
                })
        {
            return refused(UseDisposition::NotPossible);
        }
    }
    let before:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    if before
        .as_ref()
        .is_some_and(|row| !row.get("charges").is_none_or(Value::is_null))
    {
        return refused(UseDisposition::NotPossible);
    }
    let decay = native
        .item_policy(&after.production_key, &after.revision_ref)
        .and_then(|policy| {
            super::super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy).ok()
        })
        .and_then(|policy| policy.decay);
    let absolute:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_spell_item_temporal_schedules t WHERE item_instance_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=t.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries done WHERE done.item_instance_id=t.item_instance_id) FOR UPDATE")
        .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    if let Some(absolute) = absolute {
        if !rotate || before.is_some() {
            return refused(UseDisposition::NotPossible);
        }
        let Some(target) = decay.as_ref().and_then(|d| d.target.as_ref()) else {
            return refused(UseDisposition::NotPossible);
        };
        selected.effects["house_absolute"] =
            json!({"before":absolute,"target":target.production_key});
    } else if before.is_some() || decay.is_some() {
        selected.effects["house_after"] =
            json!({"key":after.production_key,"revision":after.revision_ref});
        selected.effects["house_temporal"] = json!({"before":before,"duration":decay.as_ref().map(|d|d.duration_millis),
            "target":decay.as_ref().and_then(|d|d.target.as_ref()).map(|target|&target.production_key)});
    }
    if rotate {
        selected.effects["kind"] = json!("furniture_rotation");
    }
    selected.effects["node_id"] = json!(uuid_text(*node.fact().node_id().as_bytes()));
    selected.effects["node_revision"] = json!(node.fact().registration_revision().to_string());
    selected.effects["rotate_after"] =
        json!({"key": after.production_key, "revision": after.revision_ref});
    Ok(Preparation::Ready(ActionPlan {
        recipe: selected.recipe,
        outputs: Vec::new(),
        consume_source: false,
        source_count: 0,
        effects: selected.effects,
        extra_inputs: Vec::new(),
    }))
}

pub(super) async fn apply(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    transaction: &str,
    plan: &ActionPlan,
) -> Result<(), DurabilityError> {
    if plan.effects.get("kind").and_then(Value::as_str) == Some("furniture_rotation")
        || plan.effects.get("portable_use").and_then(Value::as_bool) == Some(true)
    {
        sqlx::query("SELECT game_item_action_furniture_apply($1::uuid)")
            .bind(transaction)
            .execute(&mut ***tx)
            .await?;
        sqlx::query("SELECT game_house_furniture_absolute_apply($1::uuid)")
            .bind(transaction)
            .execute(&mut ***tx)
            .await?;
        sqlx::query("SELECT game_house_furniture_timer_apply($1::uuid)")
            .bind(transaction)
            .execute(&mut ***tx)
            .await?;
    }
    Ok(())
}
