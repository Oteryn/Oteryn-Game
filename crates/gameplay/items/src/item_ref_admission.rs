//! Pure active-generation item definition facts and index lookup.

use crate::item_transfer::BackpackEntry;
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

/// Why a durable corpse entry is left out of the corpse view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpseEntryRefusal {
    /// The pinned Item definition index does not name the entry's exact definition.
    NotInItemIndex,
    /// The entry holds no quantity.
    ZeroQuantity,
}

impl CorpseEntryRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotInItemIndex => "not_in_item_index",
            Self::ZeroQuantity => "zero_quantity",
        }
    }
}

/// A corpse entry the view shows: its wire definition reference and count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShownCorpseEntry<'a> {
    pub entry: &'a BackpackEntry,
    pub item_definition_ref: NonZeroU32,
    pub count: NonZeroU32,
}

/// A corpse entry the view leaves out, with the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefusedCorpseEntry<'a> {
    pub entry: &'a BackpackEntry,
    pub refusal: CorpseEntryRefusal,
}

/// The corpse view of the durable corpse `entries`, in loot order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CorpseView<'a> {
    pub shown: Vec<ShownCorpseEntry<'a>>,
    pub refused: Vec<RefusedCorpseEntry<'a>>,
}

/// Each entry is shown or refused on its own: one entry the pinned index does not name, or one
/// with no quantity, never hides the others (the corpse opens and shows the items it has).
pub fn corpse_view<'a>(
    index: &ItemDefinitionIndex,
    entries: &'a [BackpackEntry],
) -> CorpseView<'a> {
    let mut view = CorpseView::default();
    for entry in entries {
        let Some(item_definition_ref) = definition_ref(index, &entry.item.definition) else {
            view.refused.push(RefusedCorpseEntry {
                entry,
                refusal: CorpseEntryRefusal::NotInItemIndex,
            });
            continue;
        };
        let Some(count) = NonZeroU32::new(entry.item.quantity) else {
            view.refused.push(RefusedCorpseEntry {
                entry,
                refusal: CorpseEntryRefusal::ZeroQuantity,
            });
            continue;
        };
        view.shown.push(ShownCorpseEntry {
            entry,
            item_definition_ref,
            count,
        });
    }
    view
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

    const PRODUCTION_KEYS: &[u8] =
        include_bytes!("../../../../tools/content-schema/native-gameplay/item-keys.json");
    const LOOT_TABLES: &[u8] =
        include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json");

    /// The `(key, revision, min_count, max_count)` of every entry of the pinned loot `table`.
    fn loot_entries(
        table: &serde_json::Value,
    ) -> Result<Vec<(String, String, u64, u64)>, Box<dyn std::error::Error>> {
        table["entries"]
            .as_array()
            .ok_or("entries")?
            .iter()
            .map(|entry| {
                Ok((
                    entry["item"]["key"].as_str().ok_or("key")?.to_owned(),
                    entry["item"]["revision"]
                        .as_str()
                        .ok_or("revision")?
                        .to_owned(),
                    entry["min_count"].as_u64().ok_or("min_count")?,
                    entry["max_count"].as_u64().ok_or("max_count")?,
                ))
            })
            .collect()
    }

    fn pinned_loot_tables() -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let mut section: serde_json::Value = serde_json::from_slice(LOOT_TABLES)?;
        match section["tables"].take() {
            serde_json::Value::Array(tables) => Ok(tables),
            _ => Err("tables".into()),
        }
    }

    fn corpse_entry(ordinal: u8, key: &str, revision: &str, quantity: u32) -> BackpackEntry {
        let mut item_instance_id = [0x5c; 16];
        item_instance_id[15] = ordinal;
        BackpackEntry {
            item: crate::item_transfer::InventoryItem {
                item_instance_id,
                definition: TypedDefinitionRef {
                    family: "Item".to_owned(),
                    production_key: key.to_owned(),
                    revision_ref: revision.to_owned(),
                },
                quantity,
            },
            placement_ordinal: u64::from(ordinal) + 1,
        }
    }

    /// A Cyclops corpse holding every entry of its real pinned loot table, each at its maximum
    /// count (Crystal 201189f8 `data-otservbr-global/monster/giants/cyclops.lua`).
    fn cyclops_corpse() -> Result<Vec<BackpackEntry>, Box<dyn std::error::Error>> {
        let table = pinned_loot_tables()?
            .into_iter()
            .find(|table| table["identity"]["key"] == "oteryn:loot.creature.cyclops")
            .ok_or("cyclops loot")?;
        loot_entries(&table)?
            .into_iter()
            .enumerate()
            .map(|(ordinal, (key, revision, _, max_count))| {
                Ok(corpse_entry(
                    u8::try_from(ordinal)?,
                    &key,
                    &revision,
                    u32::try_from(max_count)?,
                ))
            })
            .collect()
    }

    /// CORPSELOOT: every entry of a real Cyclops corpse is shown, in loot order, with its count.
    #[test]
    fn cyclops_corpse_shows_every_real_loot_entry() -> Result<(), Box<dyn std::error::Error>> {
        let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS)?;
        let corpse = cyclops_corpse()?;
        assert_eq!(corpse.len(), 13);
        let view = corpse_view(&index, &corpse);
        assert!(view.refused.is_empty());
        assert_eq!(
            view.shown
                .iter()
                .map(|shown| (shown.entry, shown.count.get()))
                .collect::<Vec<_>>(),
            corpse
                .iter()
                .map(|entry| (entry, entry.item.quantity))
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    /// CORPSELOOT (E2E nbgh71): one entry the pinned index does not name, or one with no
    /// quantity, never hides the rest of the corpse; per Crystal the corpse opens and shows the
    /// items it has. Each refused entry is returned with its reason, for the caller to report.
    #[test]
    fn one_unnamed_or_empty_corpse_entry_never_hides_the_rest()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS)?;
        let real = cyclops_corpse()?;
        let mut corpse = real.clone();
        let unnamed = corpse_entry(40, "oteryn:item.tibia.i99999999", "definition-r1", 1);
        let stale_revision = corpse_entry(41, "oteryn:item.tibia.i3031", "definition-r2", 3);
        let empty = corpse_entry(42, "oteryn:item.tibia.i3031", "definition-r1", 0);
        corpse.insert(2, unnamed.clone());
        corpse.insert(5, stale_revision.clone());
        corpse.insert(9, empty.clone());
        let view = corpse_view(&index, &corpse);
        assert_eq!(
            view.shown
                .iter()
                .map(|shown| shown.entry.clone())
                .collect::<Vec<_>>(),
            real
        );
        assert_eq!(
            view.refused
                .iter()
                .map(|refused| (refused.entry.clone(), refused.refusal.reason()))
                .collect::<Vec<_>>(),
            vec![
                (unnamed, "not_in_item_index"),
                (stale_revision, "not_in_item_index"),
                (empty, "zero_quantity"),
            ]
        );
        Ok(())
    }

    /// CORPSELOOT: the whole defect class over the pinned content. Every entry of every
    /// species' loot table is an Item the pinned index names at exactly its revision, with a
    /// non-empty count range, so the corpse view refuses no real drop. A failure lists every
    /// failing species and entry.
    #[test]
    fn every_species_loot_entry_is_named_by_the_pinned_item_index()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS)?;
        let tables = pinned_loot_tables()?;
        assert_eq!(tables.len(), 1292);
        let mut failing = Vec::new();
        for table in &tables {
            let bad: Vec<String> = loot_entries(table)?
                .into_iter()
                .filter(|(key, revision, min_count, max_count)| {
                    index.definition_ref("Item", key, revision).is_none()
                        || *min_count == 0
                        || max_count < min_count
                })
                .map(|(key, _, min_count, max_count)| format!("{key}[{min_count}..={max_count}]"))
                .collect();
            if !bad.is_empty() {
                failing.push(format!("{}: {}", table["identity"]["key"], bad.join(", ")));
            }
        }
        assert!(
            failing.is_empty(),
            "species with unmappable loot: {failing:#?}"
        );
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
