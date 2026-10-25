//! CRAFT-1's basic, certain cooking conversions from the authored catalogue.
//! Unsupported operand, fluid, timed and random shapes fail closed.
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::{
    ProjectV2ConversionEffect, ProjectV2ConversionLifecycle, ProjectV2ConversionOutputs,
    ProjectV2ConversionRole, ProjectV2ConversionTarget, ProjectV2ConversionTool,
    ProjectV2Declaration, ProjectV2DefinitionRef, ReferenceItemTarget,
};
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_domain::domain::timed_item_host::resolve_timed_definition;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct CookingConversion {
    pub recipe: String,
    pub outputs: Vec<(TypedDefinitionRef, u32)>,
}

/// Resolve a fixed cooking row from server-owned data and this generation's exact item policies.
/// Each conversion consumes one USED unit and mints every output with a fresh identity.
pub fn cooking_conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    station: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        records: Vec<ProjectV2Declaration>,
    }
    static ROWS: OnceLock<Option<Vec<ProjectV2Declaration>>> = OnceLock::new();
    let rows = ROWS
        .get_or_init(|| {
            serde_json::from_str::<Catalogue>(include_str!(
                "../../../../content/craft/recipes.json"
            ))
            .ok()
            .map(|c| c.records)
        })
        .as_ref()?;
    let source_ref = |definition: &TypedDefinitionRef| {
        let record = native
            .item_policy(&definition.production_key, &definition.revision_ref)?
            .record();
        (record.production_definition.family == definition.family)
            .then_some(record.production_binding.target.clone())
    };
    let used_ref = source_ref(used)?;
    let station_ref = source_ref(station)?;
    let mut matches = rows
        .iter()
        .filter_map(|row| cooking_row(native, row, &used_ref, &station_ref));
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}

fn cooking_row(
    native: &NativeGameplayState,
    row: &ProjectV2Declaration,
    used: &ProjectV2DefinitionRef,
    station: &ProjectV2DefinitionRef,
) -> Option<CookingConversion> {
    let ProjectV2Declaration::ItemConversion {
        identity, recipe, ..
    } = row
    else {
        return None;
    };
    if recipe.input_item != *used
        || !matches!(&recipe.target, ProjectV2ConversionTarget::MapItem { definition } if definition == station)
        || recipe.tool != ProjectV2ConversionTool::None
        || recipe.skill.is_some()
        || recipe.chance_curve.is_some()
        || recipe.tile_cooldown_ms.is_some()
        || recipe.target_effect.is_some()
        || recipe.opened_target.is_some()
        || recipe.revert_ms.is_some()
        || !recipe.capacity_check
    {
        return None;
    }
    let [operand] = recipe.operands.as_slice() else {
        return None;
    };
    if operand.role != ProjectV2ConversionRole::Used
        || operand.item_id != *used
        || operand.count != 1
        || operand.sub_type.is_some()
        || operand.output_sub_type.is_some()
        || operand.consume_on.is_some()
        || operand.effect != ProjectV2ConversionEffect::TransformTo
        || operand.lifecycle != Some(ProjectV2ConversionLifecycle::ReplaceInstance)
    {
        return None;
    }
    let ProjectV2ConversionOutputs::Fixed { entries } = &recipe.outputs else {
        return None;
    };
    if entries.len() > 3
        || entries
            .iter()
            .any(|output| !(1..=100).contains(&output.count))
    {
        return None;
    }
    let output = operand.output_item_id.as_ref()?;
    let outputs = std::iter::once((output, 1))
        .chain(entries.iter().map(|o| (&o.item_id, u32::from(o.count))))
        .map(|(reference, count)| Some((output_definition(native, reference)?, count)))
        .collect::<Option<Vec<_>>>()?;
    Some(CookingConversion {
        recipe: identity.key.clone(),
        outputs,
    })
}

pub(crate) fn output_definition(
    native: &NativeGameplayState,
    reference: &ProjectV2DefinitionRef,
) -> Option<TypedDefinitionRef> {
    let id = reference
        .key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    let record = native.item_policy_for_source_id(id)?.record();
    let target = ReferenceItemTarget::new(&reference.key, &reference.revision).ok()?;
    if record.production_binding.target != *reference
        || !record.admission.materializable
        || resolve_timed_definition(&target, &record.semantics)
            .ok()?
            .is_some()
    {
        return None;
    }
    let production = &record.production_definition;
    Some(TypedDefinitionRef {
        family: production.family.clone(),
        production_key: production.production_key.clone(),
        revision_ref: production.revision_ref.clone(),
    })
}
