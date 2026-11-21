//! Exact starter custody shared by the packet journey and registered unit tests.
use super::TestResult;
use oteryn_dev_client::CharacterInventory;
use oteryn_gameplay_items::starter_grant::{NewhavenStarterDestination, newhaven_starter_items};
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, wire_slot};
use std::num::NonZeroU32;

/// The exact first-entry custody of one Character, as Item definition references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StarterLoadout {
    pub(super) backpack: NonZeroU32,
    /// `(definition, count)` directly inside the main backpack.
    pub(super) entries: Vec<(NonZeroU32, u32)>,
    pub(super) equipment: Vec<(EquipmentSlot, NonZeroU32, u32)>,
}

/// `newhaven-starter-r1` for one vocation: the main backpack holding rope and shovel ahead of
/// the vocation's Newhaven kit (`newhaven_starter_items`). Quiver contents sit inside the
/// equipped quiver, not directly in the main backpack, so they are not direct entries.
pub(super) fn newhaven_loadout(
    vocation: &str,
    reference: impl Fn(u32) -> TestResult<NonZeroU32>,
) -> TestResult<StarterLoadout> {
    let kit =
        newhaven_starter_items(vocation).ok_or("qualification vocation has no Newhaven kit")?;
    let mut entries = vec![(reference(3003)?, 1), (reference(3457)?, 1)];
    let mut equipment = Vec::new();
    for item in kit {
        let definition = reference(item.tibia_id)?;
        match item.destination {
            NewhavenStarterDestination::Backpack => entries.push((definition, item.quantity)),
            NewhavenStarterDestination::Equipment(code) => equipment.push((
                wire_slot(code).ok_or("Newhaven kit slot has no wire slot")?,
                definition,
                item.quantity,
            )),
            NewhavenStarterDestination::Quiver => {}
        }
    }
    Ok(StarterLoadout {
        backpack: reference(2854)?,
        entries,
        equipment,
    })
}

/// Wire handles are session-local. Assert the exact loadout across fresh admissions: the one
/// main backpack, every direct entry and every equipped slot, with exact counts and no extras.
pub(super) fn check_starter(
    inventory: &CharacterInventory,
    expected: &StarterLoadout,
) -> TestResult {
    let mut entries: Vec<_> = inventory
        .entries
        .iter()
        .map(|item| (item.item_definition_ref, item.count.get(), item.sub_type))
        .collect();
    let mut expected_entries: Vec<_> = expected
        .entries
        .iter()
        .map(|&(definition, count)| (definition, count, 0))
        .collect();
    let mut equipment: Vec<_> = inventory
        .equipment
        .iter()
        .map(|equipped| {
            (
                equipped.slot,
                equipped.item.item_definition_ref,
                equipped.item.count.get(),
                equipped.item.sub_type,
            )
        })
        .collect();
    let mut expected_equipment: Vec<_> = expected
        .equipment
        .iter()
        .map(|&(slot, definition, count)| (slot, definition, count, 0))
        .collect();
    entries.sort_unstable();
    expected_entries.sort_unstable();
    equipment.sort_unstable();
    expected_equipment.sort_unstable();
    if !inventory.main_backpack.is_some_and(|bag| {
        bag.item_definition_ref == expected.backpack && bag.count.get() == 1 && bag.sub_type == 0
    }) || entries != expected_entries
        || equipment != expected_equipment
    {
        return Err(format!("expected exact newhaven-starter-r1 loadout: {inventory:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view as items;

    fn reference(id: u32) -> TestResult<NonZeroU32> {
        Ok(NonZeroU32::new(id).ok_or("definition")?)
    }

    fn entry(handle: u64, definition: NonZeroU32, count: u32) -> TestResult<items::ItemEntry> {
        Ok(items::ItemEntry {
            appearance_id: 0,
            handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
            item_definition_ref: definition,
            count: NonZeroU32::new(count).ok_or("count")?,
            sub_type: 0,
        })
    }

    fn granted(expected: &StarterLoadout) -> TestResult<CharacterInventory> {
        let mut handle = 1;
        let mut next = |definition: NonZeroU32, count: u32| {
            handle += 1;
            entry(handle, definition, count)
        };
        Ok(CharacterInventory {
            main_backpack: Some(entry(1, expected.backpack, 1)?),
            entries: expected
                .entries
                .iter()
                .map(|&(definition, count)| next(definition, count))
                .collect::<TestResult<_>>()?,
            equipment: expected
                .equipment
                .iter()
                .map(
                    |&(slot, definition, count)| -> TestResult<items::EquippedItem> {
                        Ok(items::EquippedItem {
                            slot,
                            item: next(definition, count)?,
                        })
                    },
                )
                .collect::<TestResult<_>>()?,
        })
    }

    #[test]
    fn sorcerer_loadout_is_rope_shovel_and_the_newhaven_kit() -> TestResult {
        let expected = newhaven_loadout("sorcerer", reference)?;
        assert_eq!(expected.backpack.get(), 2854);
        let mut entries: Vec<_> = expected
            .entries
            .iter()
            .map(|&(definition, count)| (definition.get(), count))
            .collect();
        entries.sort_unstable();
        assert_eq!(
            entries,
            [
                (266, 2),
                (268, 10),
                (3003, 1),
                (3031, 50),
                (3457, 1),
                (3578, 1),
                (21351, 1)
            ]
        );
        let mut equipment: Vec<_> = expected
            .equipment
            .iter()
            .map(|&(slot, definition, count)| (slot, definition.get(), count))
            .collect();
        equipment.sort_unstable();
        use items::EquipmentSlot::{Ammo, Armor, Feet, Head, LeftHand, Legs, RightHand};
        assert_eq!(
            equipment,
            [
                (Head, 3355, 1),
                (Armor, 3561, 1),
                (RightHand, 21348, 1),
                (LeftHand, 21400, 1),
                (Legs, 3559, 1),
                (Feet, 3552, 1),
                (Ammo, 34017, 1)
            ]
        );
        assert!(newhaven_loadout("none", reference).is_err());
        Ok(())
    }

    #[test]
    fn paladin_quiver_contents_are_not_direct_backpack_entries() -> TestResult {
        let expected = newhaven_loadout("paladin", reference)?;
        assert_eq!(expected.entries.len(), 6);
        assert_eq!(expected.equipment.len(), 7);
        assert!(
            expected
                .entries
                .iter()
                .all(|&(definition, _)| definition.get() != 3447)
        );
        Ok(())
    }

    #[test]
    fn starter_requires_the_exact_loadout_and_no_extra_custody() -> TestResult {
        let expected = newhaven_loadout("sorcerer", reference)?;
        let exact = granted(&expected)?;
        check_starter(&exact, &expected)?;
        let mut inventory = exact.clone();
        inventory.entries.reverse();
        inventory.equipment.reverse();
        check_starter(&inventory, &expected)?;

        let other = StarterLoadout {
            backpack: reference(4)?,
            ..expected.clone()
        };
        assert!(check_starter(&exact, &other).is_err());

        let rope = exact.entries[0];
        let bag = exact.main_backpack.ok_or("bag")?;
        let mut invalid = Vec::new();
        let mut extra = exact.clone();
        extra.entries.push(rope);
        invalid.push(extra);
        let mut missing = exact.clone();
        missing.entries.remove(0);
        invalid.push(missing);
        for changed in [
            items::ItemEntry {
                item_definition_ref: bag.item_definition_ref,
                ..rope
            },
            items::ItemEntry {
                count: NonZeroU32::new(2).ok_or("two")?,
                ..rope
            },
            items::ItemEntry {
                sub_type: 1,
                ..rope
            },
        ] {
            let mut wrong = exact.clone();
            wrong.entries[0] = changed;
            invalid.push(wrong);
        }
        let mut unequipped = exact.clone();
        unequipped.equipment.pop();
        invalid.push(unequipped);
        let mut extra_equipped = exact.clone();
        extra_equipped.equipment.push(items::EquippedItem {
            slot: items::EquipmentSlot::Ring,
            item: rope,
        });
        invalid.push(extra_equipped);
        let mut wrong_slot = exact.clone();
        wrong_slot.equipment[0].slot = items::EquipmentSlot::Necklace;
        invalid.push(wrong_slot);
        for main_backpack in [
            Some(items::ItemEntry {
                count: NonZeroU32::new(2).ok_or("two")?,
                ..bag
            }),
            None,
            Some(items::ItemEntry { sub_type: 1, ..bag }),
        ] {
            invalid.push(CharacterInventory {
                main_backpack,
                ..exact.clone()
            });
        }
        for inventory in invalid {
            assert!(
                check_starter(&inventory, &expected).is_err(),
                "{inventory:?}"
            );
        }
        Ok(())
    }
}
