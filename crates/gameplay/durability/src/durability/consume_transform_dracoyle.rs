//! The statue is the primary operand; one exact carried/nested emerald is its cost.
use super::*;

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
) -> Result<Option<ActionPlan>, DurabilityError> {
    if !matches!(
        definition.production_key.as_str(),
        "oteryn:item.tibia.i9035" | "oteryn:item.tibia.i9040"
    ) {
        return Ok(None);
    }
    let Some(gem) = request.target.filter(|id| *id != request.source) else {
        return Ok(None);
    };
    let Some(input) =
        mount_kits::carried_item(tx, fence, Some(gem), "oteryn:item.tibia.i677", native).await?
    else {
        return Ok(None);
    };
    let gem_definition = TypedDefinitionRef {
        family: input.before["definition_family"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
        production_key: input.before["definition_production_key"]
            .as_str()
            .ok_or(DurabilityError::InvalidStoredState)?
            .into(),
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
    let target = if definition.production_key == "oteryn:item.tibia.i9035" {
        9035
    } else {
        9040
    };
    Ok(Some(ActionPlan {
        recipe: conversion.recipe,
        outputs: conversion.outputs,
        consume_source: true,
        source_count: 1,
        effects: json!({"kind":"dracoyle","source_item_id":target,"timed_output":{"duration_ms":604800000_u64,"decay_to":target},"effect":18}),
        extra_inputs: vec![input],
    }))
}
