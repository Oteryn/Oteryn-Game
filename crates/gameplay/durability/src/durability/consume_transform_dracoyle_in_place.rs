//! Actual Ground/House statue custody is retained; one actual carried emerald is spent.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeItemStackClass;

pub(super) async fn prepare_in_place(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    ground: Option<oteryn_gameplay_items::novelty_items::GroundEffect>,
) -> Result<Option<ActionPlan>, DurabilityError> {
    let inactive = match definition.production_key.as_str() {
        "oteryn:item.tibia.i9035" => 9035,
        "oteryn:item.tibia.i9040" => 9040,
        _ => return Ok(None),
    };
    let Some(gem) = request.target.filter(|id| *id != request.source) else {
        return Ok(None);
    };
    let Some(input) =
        mount_kits::carried_item(tx, fence, Some(gem), "oteryn:item.tibia.i677", native).await?
    else {
        return Ok(None);
    };
    let gem_definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i677".into(),
        revision_ref: input.before["definition_revision_ref"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
    };
    let Some(conversion) = oteryn_gameplay_items::item_craft::enchanting::dracoyle_conversion(
        native,
        &gem_definition,
        definition,
    ) else {
        return Ok(None);
    };
    let [(after, 1)] = conversion.outputs.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let policy = native
        .item_policy(&after.production_key, &after.revision_ref)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if policy.record().admission.stack_class != NativeItemStackClass::NonStackable {
        return Ok(None);
    };
    let decay = super::super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .decay
        .ok_or(DurabilityError::InvalidStoredState)?;
    if decay.duration_millis != 604800000 || decay.target.as_ref() != Some(definition) {
        return Err(DurabilityError::InvalidStoredState);
    };
    let effects=oteryn_gameplay_items::novelty_items::NoveltyEffects{
  kind:oteryn_gameplay_items::novelty_items::NoveltyKind::Transform,source_item_id:inactive,
  donor:"crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/enchanting.lua:107-112".into(),
  ground,cues:vec![oteryn_gameplay_items::novelty_items::NoveltyCue{binding:"appearance:effect/magic_green".into(),effect_id:18}],
  speech:None,private_speech:false,require_head:false,require_player_target:false,dice_value:None,costume:None,
  damage:None,achievement:None,achievement_progress:vec![],timed_output:None,
 };
    let timer:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE").bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    let live_absolute:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts d WHERE d.item_instance_id=p.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries d WHERE d.item_instance_id=p.item_instance_id))").bind(request.source.as_slice()).fetch_one(&mut ***tx).await?;
    if live_absolute
        || timer
            .as_ref()
            .is_some_and(|t| !t["charges"].is_null() || !t["deadline_at"].is_null())
    {
        return Ok(None);
    }
    let mut effects =
        serde_json::to_value(effects).map_err(|_| DurabilityError::InvalidStoredState)?;
    effects["house_after"] = json!({"key":after.production_key,"revision":after.revision_ref});
    effects["house_temporal"] =
        json!({"before":timer,"duration":604800000_u64,"target":definition.production_key});
    effects["dracoyle_node"] = json!({"id":uuid_text(*node.fact().node_id().as_bytes()),"revision":node.fact().registration_revision().to_string()});
    effects["dracoyle_in_place"] = json!(true);
    effects["dracoyle_after"] = json!({"key":after.production_key,"revision":after.revision_ref});
    effects["dracoyle_duration_ms"] = json!(604800000_u64);
    effects["dracoyle_decay_to"] = json!(inactive);
    Ok(Some(ActionPlan {
        recipe: conversion.recipe,
        outputs: Vec::new(),
        consume_source: false,
        source_count: 0,
        effects,
        extra_inputs: vec![input],
    }))
}
