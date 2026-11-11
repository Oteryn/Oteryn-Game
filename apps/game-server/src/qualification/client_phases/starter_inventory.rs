//! Exact starter custody shared by the packet journey and registered unit tests.
use super::TestResult;
use oteryn_dev_client::CharacterInventory;
use std::num::NonZeroU32;

/// Wire handles are session-local. Assert the exact no-vocation `nb-starter-1` loadout
/// across fresh admissions, including both backpack entries and empty equipment.
pub(super) fn check_starter(
    inventory: &CharacterInventory,
    definition: NonZeroU32,
    entry_definitions: [NonZeroU32; 2],
) -> TestResult {
    let mut entries: Vec<_> = inventory
        .entries
        .iter()
        .map(|item| (item.item_definition_ref, item.count.get(), item.sub_type))
        .collect();
    let mut expected = entry_definitions.map(|definition| (definition, 1, 0));
    entries.sort_unstable();
    expected.sort_unstable();
    if !inventory.main_backpack.is_some_and(|bag| {
        bag.item_definition_ref == definition && bag.count.get() == 1 && bag.sub_type == 0
    }) || entries.as_slice() != expected.as_slice()
        || !inventory.equipment.is_empty()
    {
        return Err(format!("expected exact nb-starter-1 loadout: {inventory:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view as items;

    #[test]
    fn starter_requires_exact_single_backpack_and_no_extra_custody() -> TestResult {
        let definition = NonZeroU32::new(1).ok_or("definition")?;
        let entry_definitions = [
            NonZeroU32::new(2).ok_or("rope definition")?,
            NonZeroU32::new(3).ok_or("shovel definition")?,
        ];
        let bag = items::ItemEntry {
            handle: std::num::NonZeroU64::new(1).ok_or("handle")?,
            item_definition_ref: definition,
            count: NonZeroU32::new(1).ok_or("count")?,
            sub_type: 0,
        };
        let rope = items::ItemEntry {
            handle: std::num::NonZeroU64::new(2).ok_or("rope handle")?,
            item_definition_ref: entry_definitions[0],
            ..bag
        };
        let shovel = items::ItemEntry {
            handle: std::num::NonZeroU64::new(3).ok_or("shovel handle")?,
            item_definition_ref: entry_definitions[1],
            ..bag
        };
        let mut inventory = CharacterInventory {
            main_backpack: Some(bag),
            entries: vec![rope, shovel],
            ..Default::default()
        };
        check_starter(&inventory, definition, entry_definitions)?;
        inventory.entries.reverse();
        check_starter(&inventory, definition, entry_definitions)?;
        assert!(
            check_starter(
                &inventory,
                NonZeroU32::new(4).ok_or("other definition")?,
                entry_definitions,
            )
            .is_err()
        );
        inventory.entries.push(bag);
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.entries = vec![rope];
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.entries.clear();
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.entries = vec![rope, rope];
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        for invalid in [
            items::ItemEntry {
                item_definition_ref: definition,
                ..rope
            },
            items::ItemEntry {
                count: NonZeroU32::new(2).ok_or("two entries")?,
                ..rope
            },
            items::ItemEntry {
                sub_type: 1,
                ..rope
            },
        ] {
            inventory.entries = vec![invalid, shovel];
            assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        }
        inventory.entries = vec![rope, shovel];
        inventory.equipment.push(items::EquippedItem {
            slot: items::EquipmentSlot::RightHand,
            item: bag,
        });
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.equipment.clear();
        inventory.main_backpack = Some(items::ItemEntry {
            count: NonZeroU32::new(2).ok_or("two")?,
            ..bag
        });
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.main_backpack = None;
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        inventory.main_backpack = Some(items::ItemEntry { sub_type: 1, ..bag });
        assert!(check_starter(&inventory, definition, entry_definitions).is_err());
        Ok(())
    }
}
