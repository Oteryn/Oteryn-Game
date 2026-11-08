//! CRAFT-1's basic, certain cooking conversions from the authored catalogue.
//! Unsupported operand, fluid, timed and random shapes fail closed.
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::{
    ProjectV2ConversionEffect, ProjectV2ConversionLifecycle, ProjectV2ConversionOperand,
    ProjectV2ConversionOutputs, ProjectV2ConversionRole, ProjectV2ConversionTarget,
    ProjectV2ConversionTool, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family,
    ReferenceItemTarget,
};
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_domain::domain::timed_item_host::resolve_timed_definition;
use std::sync::OnceLock;

mod captured_target;
pub mod enchanting;
mod quest_recipes;
pub use captured_target::{TAINTED_SOULS_RECIPE, captured_target_quantity};

#[derive(Debug)]
pub struct CookingConversion {
    pub recipe: String,
    pub outputs: Vec<(TypedDefinitionRef, u32)>,
}

/// A conversion resolved against the exact locked carried target stack.
#[derive(Debug)]
pub struct CarriedCookingConversion {
    pub conversion: CookingConversion,
    pub target_debit: u32,
}

/// Resolve Crystal's two-carried-item food preparation rows. Both operands are consumed by the
/// durable owner and every replacement/output is minted into the source backpack.
pub fn carried_cooking_conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    carried_cooking_conversion_with_sub_type(native, used, target, None)
}

/// Resolve a carried conversion whose target may encode a fluid subtype (water `1`, milk `9`).
pub fn carried_cooking_conversion_with_sub_type(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
    target_sub_type: Option<u32>,
) -> Option<CookingConversion> {
    carried_cooking_conversion_with_quantity(native, used, target, target_sub_type, 1)
        .map(|resolved| resolved.conversion)
}

/// Resolve capture from the durable owner's locked quantity, never a client count.
pub fn carried_cooking_conversion_with_quantity(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
    target_sub_type: Option<u32>,
    target_quantity: u32,
) -> Option<CarriedCookingConversion> {
    if !(1..=100).contains(&target_quantity) {
        return None;
    }
    if let Some(conversion) = quest_recipes::carried(native, used, target) {
        return Some(CarriedCookingConversion {
            conversion,
            target_debit: 1,
        });
    }
    let rows = catalogue_rows()?;
    let used_ref = source_ref(native, used)?;
    let target_ref = source_ref(native, target)?;
    let mut matches = rows.iter().filter_map(|row| {
        carried_row(
            native,
            row,
            &used_ref,
            &target_ref,
            target_sub_type,
            target_quantity,
        )
    });
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}

/// The first reachable Global fishing slice: a normal rod and worm on an admitted water tile
/// consumes one worm and produces one fish. Dispatch owns the skill-scaled catch decision;
/// durability revalidates all three definitions before mutating inventory.
pub fn fishing_conversion(
    native: &NativeGameplayState,
    tool: &TypedDefinitionRef,
    bait: &TypedDefinitionRef,
    water: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    use crate::fishing::{FISH, FISHING_ROD, WORM, is_water};
    if tool.production_key != FISHING_ROD
        || bait.production_key != WORM
        || !is_water(&water.production_key)
    {
        return None;
    }
    let output = native.item_policy_for_source_id(3578)?.record();
    (output.production_definition.production_key == FISH).then(|| CookingConversion {
        recipe: "oteryn:fishing.global.standard".into(),
        outputs: vec![(
            TypedDefinitionRef {
                family: output.production_definition.family.clone(),
                production_key: output.production_definition.production_key.clone(),
                revision_ref: output.production_definition.revision_ref.clone(),
            },
            1,
        )],
    })
}

/// Resolve a fixed cooking row from server-owned data and this generation's exact item policies.
/// Each conversion consumes one USED unit and mints every output with a fresh identity.
pub fn cooking_conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    station: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    let rows = catalogue_rows()?;
    let used_ref = source_ref(native, used)?;
    let station_ref = source_ref(native, station)?;
    let mut matches = rows
        .iter()
        .filter_map(|row| cooking_row(native, row, &used_ref, &station_ref));
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}

fn catalogue_rows() -> Option<&'static [ProjectV2Declaration]> {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        records: Vec<ProjectV2Declaration>,
    }
    static ROWS: OnceLock<Option<Vec<ProjectV2Declaration>>> = OnceLock::new();
    ROWS.get_or_init(|| {
        serde_json::from_str::<Catalogue>(include_str!("../../../../content/craft/recipes.json"))
            .ok()
            .map(|c| c.records)
    })
    .as_deref()
}

fn source_ref(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<ProjectV2DefinitionRef> {
    if let Some(policy) = native.item_policy(&definition.production_key, &definition.revision_ref) {
        let record = policy.record();
        return (record.production_definition.family == definition.family)
            .then_some(record.production_binding.target.clone());
    }
    // Some donor ingredients (notably the garlic bulb) are identity-only in the current
    // catalogue. They may be consumed when already carried, but are never minted here.
    (definition.family == "Item"
        && definition.production_key.starts_with("oteryn:item.tibia.i")
        && definition.revision_ref == "definition-r1")
        .then(|| ProjectV2DefinitionRef {
            family: ProjectV2Family::Item,
            key: definition.production_key.clone(),
            revision: definition.revision_ref.clone(),
        })
}

fn carried_row(
    native: &NativeGameplayState,
    row: &ProjectV2Declaration,
    used: &ProjectV2DefinitionRef,
    target: &ProjectV2DefinitionRef,
    target_sub_type: Option<u32>,
    target_quantity: u32,
) -> Option<CarriedCookingConversion> {
    let ProjectV2Declaration::ItemConversion {
        identity, recipe, ..
    } = row
    else {
        return None;
    };
    if recipe.input_item != *used
        || !matches!(recipe.target, ProjectV2ConversionTarget::Carried)
        || recipe.tool != ProjectV2ConversionTool::None
        || recipe.skill.is_some()
        || recipe.chance_curve.is_some()
        || !recipe.capacity_check
    {
        return None;
    }
    let [used_operand, target_operand] = recipe.operands.as_slice() else {
        return None;
    };
    if used_operand.role != ProjectV2ConversionRole::Used
        || used_operand.item_id != *used
        || used_operand.count != 1
        || target_operand.role != ProjectV2ConversionRole::Target
        || target_operand.item_id != *target
        || target_operand.count != 1
        || target_operand
            .sub_type
            .is_some_and(|required| Some(u32::from(required)) != target_sub_type)
    {
        return None;
    }
    if !matches!(
        used_operand.effect,
        ProjectV2ConversionEffect::Consume | ProjectV2ConversionEffect::TransformTo
    ) || !matches!(
        target_operand.effect,
        ProjectV2ConversionEffect::Consume | ProjectV2ConversionEffect::TransformTo
    ) {
        return None;
    }
    let target_debit = carried_target_debit(target_operand, target_quantity)?;
    let mut outputs = Vec::new();
    for operand in [used_operand, target_operand] {
        if operand.effect == ProjectV2ConversionEffect::TransformTo {
            let definition = output_definition(native, operand.output_item_id.as_ref()?)?;
            let count = if operand.capture_target_stack {
                // A nonstacking definition represents one transformed object. Stackable
                // replacements retain the captured count; unknown stack policy fails closed.
                use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;
                match crate::item_ref_admission::generation_item_facts(native, &definition)
                    .ok()?
                    .stack
                {
                    ItemStackClass::NonStackable => 1,
                    ItemStackClass::Stackable {
                        proven_maximum: Some(maximum),
                    } if target_debit <= maximum => target_debit,
                    _ => return None,
                }
            } else {
                1
            };
            outputs.push((definition, count));
        }
    }
    let ProjectV2ConversionOutputs::Fixed { entries } = &recipe.outputs else {
        return None;
    };
    for output in entries {
        outputs.push((
            output_definition(native, &output.item_id)?,
            u32::from(output.count),
        ));
    }
    (!outputs.is_empty() && outputs.len() <= 3).then(|| CarriedCookingConversion {
        conversion: CookingConversion {
            recipe: identity.key.clone(),
            outputs,
        },
        target_debit,
    })
}

fn carried_target_debit(operand: &ProjectV2ConversionOperand, quantity: u32) -> Option<u32> {
    (operand.role == ProjectV2ConversionRole::Target && (1..=100).contains(&quantity)).then_some(
        if operand.capture_target_stack {
            quantity
        } else {
            1
        },
    )
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

pub fn output_definition(
    native: &NativeGameplayState,
    reference: &ProjectV2DefinitionRef,
) -> Option<TypedDefinitionRef> {
    let id = reference
        .key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    let Some(policy) = native.item_policy_for_source_id(id) else {
        // The loot owner is the other pinned source of materializable Item admission. Merely
        // appearing in the key index is not permission to mint the definition.
        let admitted = native.reward_table().admitted_source_item(id)?;
        return (admitted.family == "Item"
            && admitted.production_key == reference.key
            && admitted.revision_ref == reference.revision)
            .then(|| TypedDefinitionRef {
                family: admitted.family.clone(),
                production_key: admitted.production_key.clone(),
                revision_ref: admitted.revision_ref.clone(),
            });
    };
    let record = policy.record();
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

#[cfg(test)]
mod target_stack_tests {
    use super::*;
    use oteryn_gameplay_content::content::ProjectV2ConversionOperand;

    #[test]
    fn m22_captured_target_debit_uses_exact_bounded_stack() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut operand: ProjectV2ConversionOperand = serde_json::from_value(serde_json::json!({
            "role": "TARGET", "item_id": {"family":"Item", "key":"oteryn:item.tibia.i10297", "revision":"definition-r1"},
            "count":1, "effect":"CONSUME", "capture_target_stack":true
        }))?;
        for quantity in [1, 7, 100] {
            assert_eq!(carried_target_debit(&operand, quantity), Some(quantity));
        }
        for quantity in [0, 101, u32::MAX] {
            assert_eq!(carried_target_debit(&operand, quantity), None);
        }
        operand.capture_target_stack = false;
        assert_eq!(carried_target_debit(&operand, 7), Some(1));
        Ok(())
    }
}
