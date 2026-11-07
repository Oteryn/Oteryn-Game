//! A carried, unchanged tool applied to one plain replacement material.
use super::*;

/// Resolve the authored USED=KEEP / TARGET=TRANSFORM_TO shape. The durable craft
/// owner consumes the material as its source and locks the unchanged tool through
/// its existing tool participant. No new consumption or MINT authority is added.
pub fn retained_carried_tool_conversion(
    native: &NativeGameplayState,
    tool: &TypedDefinitionRef,
    material: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    let tool_ref = source_ref(native, tool)?;
    let material_ref = source_ref(native, material)?;
    if output_definition(native, &tool_ref).as_ref() != Some(tool)
        || output_definition(native, &material_ref).as_ref() != Some(material)
    {
        return None;
    }
    let mut matches = catalogue_rows()?.iter().filter_map(|row| {
        let ProjectV2Declaration::ItemConversion { identity, recipe, .. } = row else {
            return None;
        };
        let [used, target] = recipe.operands.as_slice() else { return None; };
        if recipe.input_item != tool_ref
            || !matches!(recipe.target, ProjectV2ConversionTarget::Carried)
            || recipe.tool != ProjectV2ConversionTool::None
            || recipe.skill.is_some() || recipe.chance_curve.is_some()
            || recipe.tile_cooldown_ms.is_some() || recipe.revert_ms.is_some()
            || recipe.opened_target.is_some() || recipe.target_effect.is_some()
            || !recipe.capacity_check
            || used.role != ProjectV2ConversionRole::Used || used.item_id != tool_ref
            || used.count != 1 || used.effect != ProjectV2ConversionEffect::Keep
            || used.sub_type.is_some() || used.output_sub_type.is_some()
            || used.consume_on.is_some() || used.output_item_id.is_some() || used.lifecycle.is_some()
            || target.role != ProjectV2ConversionRole::Target || target.item_id != material_ref
            || target.count != 1 || target.effect != ProjectV2ConversionEffect::TransformTo
            || target.lifecycle != Some(ProjectV2ConversionLifecycle::ReplaceInstance)
            || target.sub_type.is_some() || target.output_sub_type.is_some() || target.consume_on.is_some()
            || !matches!(&recipe.outputs, ProjectV2ConversionOutputs::Fixed { entries } if entries.is_empty())
        { return None; }
        Some(CookingConversion {
            recipe: identity.key.clone(),
            outputs: vec![(output_definition(native, target.output_item_id.as_ref()?)?, 1)],
        })
    });
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}
