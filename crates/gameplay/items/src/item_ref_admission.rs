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
    generation_destination_facts(
        native,
        definition,
        NativeItemDestination::CharacterInventory,
    )
}

/// Native Inbox may receive a carried Item or a definition with an explicit
/// Inbox-only admission. This does not make that definition portable. Content
/// qualification closes the new destination to the immovable Hireling lamp.
pub fn generation_inbox_item_facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Result<ItemDefinitionFacts, PickupContentError> {
    generation_destination_facts(native, definition, NativeItemDestination::CharacterInbox)
}

/// Facts for relocation on Ground. This grants no carried-inventory admission.
pub fn generation_ground_item_facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Result<ItemDefinitionFacts, PickupContentError> {
    generation_destination_facts(native, definition, NativeItemDestination::Ground)
}

fn generation_destination_facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
    destination: NativeItemDestination,
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
    profile_item_facts(record, definition, destination)
}

fn profile_item_facts(
    record: &oteryn_gameplay_content::content::native_gameplay::ItemProfileRecord,
    definition: &TypedDefinitionRef,
    destination: NativeItemDestination,
) -> Result<ItemDefinitionFacts, PickupContentError> {
    let legal = &record.admission.legal_destinations;
    let admitted = record.admission.materializable
        && match destination {
            NativeItemDestination::CharacterInventory => legal.contains(&destination),
            NativeItemDestination::CharacterInbox => {
                legal.contains(&destination)
                    || legal.contains(&NativeItemDestination::CharacterInventory)
            }
            NativeItemDestination::Ground => {
                legal.contains(&destination)
                    && matches!(
                        &record.semantics.physical,
                        ReferenceItemField::Known(physical) if physical.movable == ReferenceItemField::Known(true)
                    )
            }
        };
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
    fn native_crocodile_steak_is_admitted_to_character_inventory()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::ReferenceItemConsumption;
        use oteryn_gameplay_content::content::native_gameplay::ItemProfileRecord;

        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i10219".into(),
            revision_ref: "definition-r1".into(),
        };
        let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/items/definitions/spell-native-profiles.json"
        )))?;
        let value = document["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|row| row["production_definition"]["production_key"] == definition.production_key)
            .ok_or("active crocodile steak profile")?;
        let record: ItemProfileRecord = serde_json::from_value(value.clone())?;
        let facts = profile_item_facts(
            &record,
            &definition,
            NativeItemDestination::CharacterInventory,
        )?;
        assert_eq!(
            facts.stack,
            ItemStackClass::Stackable {
                proven_maximum: Some(100)
            }
        );
        assert!(matches!(
            record.semantics.consumption,
            ReferenceItemField::Known(ReferenceItemConsumption::Food(food))
                if food.regeneration_seconds == 120
        ));
        Ok(())
    }

    #[test]
    fn active_item_generation_resolves_exact_portable_items_and_corpse_states()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = oteryn_gameplay_content::content::native_gameplay_test_world_id()?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let native =
            oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            );
        for (id, portable) in [
            (836, true),
            (860, true),
            (3994, true),
            (3995, true),
            (3996, true),
            (5678, true),
            (5949, true),
            (5964, false),
            (6125, true),
            (6278, true),
            (6392, true),
            (7373, true),
            (7374, true),
            (7376, true),
            (8012, true),
            (8177, true),
            (8747, true),
            (8862, true),
            (8864, true),
            (54373, false),
            (54374, false),
            (54375, false),
            (54376, false),
            (54396, false),
            (54397, false),
            (54398, false),
            (54399, false),
            (54412, false),
            (54413, false),
            (54414, false),
            (54415, false),
            (54568, false),
            (54569, false),
            (54570, false),
            (54571, false),
            (54678, false),
            (54679, false),
            (54680, false),
            (54681, false),
        ] {
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            };
            let facts = generation_item_facts(&native, &definition)
                .map_err(|error| format!("{id}: {error:?}"))?;
            assert_eq!(facts.stack != ItemStackClass::Unknown, portable, "{id}");
        }
        Ok(())
    }

    #[test]
    fn native_amphora_ground_facts_do_not_grant_inventory_or_immovable_admission()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::native_gameplay::ItemProfileRecord;
        let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/items/definitions/spell-native-profiles.json"
        )))?;
        let value = document["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i2904")
            .ok_or("native amphora")?;
        let mut record: ItemProfileRecord = serde_json::from_value(value.clone())?;
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2904".into(),
            revision_ref: "definition-r1".into(),
        };
        let ground = profile_item_facts(&record, &definition, NativeItemDestination::Ground)?;
        assert_eq!(
            ground.stack,
            ItemStackClass::NonStackable,
            "the native movable amphora must relocate on Ground"
        );
        assert_eq!(
            profile_item_facts(
                &record,
                &definition,
                NativeItemDestination::CharacterInventory
            )?
            .stack,
            ItemStackClass::Unknown
        );
        assert_eq!(
            profile_item_facts(&record, &definition, NativeItemDestination::CharacterInbox)?.stack,
            ItemStackClass::Unknown
        );
        if let ReferenceItemField::Known(physical) = &mut record.semantics.physical {
            physical.movable = ReferenceItemField::Known(false);
        }
        assert_eq!(
            profile_item_facts(&record, &definition, NativeItemDestination::Ground)?.stack,
            ItemStackClass::Unknown
        );
        Ok(())
    }
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
