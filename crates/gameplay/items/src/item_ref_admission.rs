//! Pure active-generation item definition facts and index lookup.

use crate::pickup::PickupContentError;
use oteryn_gameplay_content::content::item_ref::ItemDefinitionIndex;
use oteryn_gameplay_content::content::native_gameplay::{
    NativeGameplayState, NativeItemDestination, NativeItemStackClass,
};
use oteryn_gameplay_content::content::{ReferenceEquipmentSlot, ReferenceItemField};
use oteryn_gameplay_domain::domain::item_definition::{
    ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef,
};
use std::num::NonZeroU32;

pub fn definition_ref(
    index: &ItemDefinitionIndex,
    definition: &TypedDefinitionRef,
) -> Option<NonZeroU32> {
    index.definition_ref(
        &definition.family,
        &definition.production_key,
        &definition.revision_ref,
    )
}

/// The D82/D83/GAME-ITEM-01 §6.2 facts of `definition`, read from the active generation's
/// canonical Item profiles (Codex 4202915395: never from the entry-room fixture catalogue). Only a
/// profile whose production definition is exactly `definition` counts. A definition the
/// generation does not profile is `DefinitionNotFound`; one it does not admit into a
/// character's inventory has the `Unknown` stack class, which the TRANSFER refuses (D82).
pub fn generation_item_facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Result<ItemDefinitionFacts, PickupContentError> {
    let record = native
        .item_policy(&definition.production_key, &definition.revision_ref)
        .map(|policy| policy.record())
        .filter(|record| {
            let production = &record.production_definition;
            production.family == definition.family
                && production.production_key == definition.production_key
                && production.revision_ref == definition.revision_ref
        })
        .ok_or(PickupContentError::DefinitionNotFound)?;
    let admitted = record.admission.materializable
        && record
            .admission
            .legal_destinations
            .contains(&NativeItemDestination::CharacterInventory);
    let stack = match record.admission.stack_class {
        _ if !admitted => ItemStackClass::Unknown,
        NativeItemStackClass::NonStackable => ItemStackClass::NonStackable,
        NativeItemStackClass::StackCapable => ItemStackClass::Stackable {
            proven_maximum: match &record.semantics.stack {
                ReferenceItemField::Known(stack) => match stack.stack_max {
                    ReferenceItemField::Known(maximum) => Some(u32::from(maximum)),
                    _ => None,
                },
                _ => None,
            },
        },
    };
    // As `combat_pickup::resolve_item_definition_facts`: only a KNOWN capacity makes a container,
    // and only a KNOWN pattern whose primary slot is `container` is the container-slot pattern.
    let container_capacity = match &record.semantics.container {
        ReferenceItemField::Known(container) => match container.capacity {
            ReferenceItemField::Known(capacity) => Some(u32::from(capacity)),
            _ => None,
        },
        _ => None,
    };
    let container_slot_equip_pattern = matches!(
        &record.semantics.equipment,
        ReferenceItemField::Known(equipment)
            if matches!(
                &equipment.patterns,
                ReferenceItemField::Known(patterns)
                    if patterns.iter().any(|pattern| {
                        pattern.primary_slot
                            == ReferenceItemField::Known(ReferenceEquipmentSlot::Container)
                    })
            )
    );
    // D1211: native weights are already normalized centi-ounces. Unknown physical
    // semantics or weight must remain unknown; never infer zero or convert again.
    let weight = normalized_item_weight(&record.semantics.physical);

    Ok(ItemDefinitionFacts {
        definition: definition.clone(),
        weight,
        stack,
        container_capacity,
        container_slot_equip_pattern,
    })
}

fn normalized_item_weight(
    physical: &ReferenceItemField<oteryn_gameplay_content::content::ReferenceItemPhysical>,
) -> Option<u32> {
    match physical {
        ReferenceItemField::Known(physical) => match physical.weight {
            ReferenceItemField::Known(weight) => Some(weight),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_weight_normalization_never_guesses_unknown_physical_or_weight() {
        use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
        use oteryn_gameplay_content::content::ReferenceItemPhysical;

        for physical in [Unknown, Conflict, NotApplicable] {
            assert_eq!(normalized_item_weight(&physical), None);
        }
        for weight in [Unknown, Conflict, NotApplicable] {
            let physical = Known(ReferenceItemPhysical {
                weight,
                movable: Known(true),
                pickupable: Known(true),
            });
            assert_eq!(normalized_item_weight(&physical), None);
        }
        for weight in [0, 10, 1_800, u32::MAX] {
            let physical = Known(ReferenceItemPhysical {
                weight: Known(weight),
                movable: Known(true),
                pickupable: Known(true),
            });
            assert_eq!(normalized_item_weight(&physical), Some(weight));
        }
    }
}
