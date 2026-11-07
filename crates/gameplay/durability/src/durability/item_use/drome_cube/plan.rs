//! PREPARE of the mandatory carried output; no ground or missing-output fallback.
use super::*;
use crate::durability::item_transfer::ItemDefinitionFacts;

/// Use the declared stack class, not the potion-flask planner's unconditional 100-unit
/// merge. A full/non-stackable/reserved receiver can still leave room for a fresh item.
pub fn plan_output(
    inventory: &ItemUseInventory,
    source: &BackpackEntry,
    output: ItemDefinitionFacts,
    capacity: u32,
    unavailable: &[[u8; 16]],
    fresh_id: impl FnOnce() -> Option<[u8; 16]>,
) -> Result<ItemUseFlaskPlan> {
    if output.definition.family != "Item" || output.container_capacity.is_some() {
        return Err(ItemUseError::InvalidInput.into());
    }
    let (maximum, stackable) =
        stack_maximum(output.stack).map_err(|_| ItemUseError::InvalidInput)?;
    let bag = inventory
        .backpack
        .as_ref()
        .ok_or(ItemUseError::StaleState)?;
    if stackable
        && let Some(entry) = bag.entries.iter().find(|entry| {
            entry.item.definition == output.definition
                && (1..maximum).contains(&entry.item.quantity)
                && !unavailable.contains(&entry.item.item_instance_id)
        })
    {
        let plan = ItemUseFlaskPlan::Stack {
            item_instance_id: entry.item.item_instance_id,
            definition: output.definition,
            quantity_before: entry.item.quantity,
        };
        check_room(inventory, source, capacity, false)?;
        return Ok(plan);
    }
    let ordinal = bag
        .entries
        .iter()
        .map(|entry| entry.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(ItemUseError::InvalidInput)?;
    check_room(inventory, source, capacity, true)?;
    Ok(ItemUseFlaskPlan::New {
        item_instance_id: fresh_id().ok_or(ItemUseError::InvalidInput)?,
        definition: output.definition,
        placement_ordinal: ordinal,
    })
}

pub(super) fn check_room(
    inventory: &ItemUseInventory,
    source: &BackpackEntry,
    capacity: u32,
    fresh: bool,
) -> Result<()> {
    let bag = inventory
        .backpack
        .as_ref()
        .ok_or(ItemUseError::StaleState)?;
    let retires = source.item.quantity == 1;
    let main = bag
        .entries
        .iter()
        .any(|entry| entry.item.item_instance_id == source.item.item_instance_id);
    let capacity = usize::try_from(capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX))
        .map_err(|_| ItemUseError::InvalidInput)?;
    let main_after = bag
        .entries
        .len()
        .checked_add(usize::from(fresh))
        .and_then(|count| count.checked_sub(usize::from(retires && main)))
        .ok_or(ItemUseError::InvalidInput)?;
    let tree_after = bag
        .entries
        .len()
        .checked_add(inventory.nested.len())
        .and_then(|count| count.checked_add(usize::from(fresh)))
        .and_then(|count| count.checked_sub(usize::from(retires)))
        .ok_or(ItemUseError::InvalidInput)?;
    if capacity == 0 || main_after > capacity || tree_after > 500 {
        return Err(DromeCubeError::Capacity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::item_transfer::{CharacterBackpack, InventoryItem};
    use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn entry(id: u8, key: &str, quantity: u32) -> BackpackEntry {
        BackpackEntry {
            item: InventoryItem {
                item_instance_id: [id; 16],
                definition: definition(key),
                quantity,
            },
            placement_ordinal: u64::from(id),
        }
    }

    fn inventory(entries: Vec<BackpackEntry>) -> ItemUseInventory {
        ItemUseInventory {
            content_revision: "content-r1".into(),
            ruleset_revision: "ruleset-r1".into(),
            equipment: Vec::new(),
            backpack: Some(CharacterBackpack {
                backpack: entry(50, "bag", 1).item,
                entries,
                fluids: Default::default(),
            }),
            nested: Vec::new(),
            keys: Vec::new(),
            fluids: Default::default(),
        }
    }

    fn facts(stack: ItemStackClass) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            weight: Some(100),
            definition: definition("output"),
            stack,
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }

    #[test]
    fn non_stackable_output_never_grows_an_existing_item() {
        let source = entry(1, DROME_CUBE_KEY, 1);
        let inventory = inventory(vec![source.clone(), entry(2, "output", 1)]);
        let plan = plan_output(
            &inventory,
            &source,
            facts(ItemStackClass::NonStackable),
            20,
            &[],
            || Some([3; 16]),
        );
        assert!(matches!(
            plan,
            Ok(ItemUseFlaskPlan::New {
                item_instance_id: [3, ..],
                ..
            })
        ));
    }

    #[test]
    fn full_or_reserved_receivers_do_not_prevent_a_fresh_output() {
        let source = entry(1, DROME_CUBE_KEY, 1);
        for (quantity, unavailable, top_up) in [
            (24, vec![], true),
            (25, vec![], false),
            (24, vec![[2; 16]], false),
        ] {
            let inventory = inventory(vec![source.clone(), entry(2, "output", quantity)]);
            let plan = plan_output(
                &inventory,
                &source,
                facts(ItemStackClass::Stackable {
                    proven_maximum: Some(25),
                }),
                20,
                &unavailable,
                || Some([3; 16]),
            );
            assert!(plan.is_ok());
            assert_eq!(matches!(plan, Ok(ItemUseFlaskPlan::Stack { .. })), top_up);
        }
    }

    #[test]
    fn retiring_a_nested_cube_does_not_free_a_main_backpack_slot() {
        let source = entry(1, DROME_CUBE_KEY, 1);
        let mut inventory = inventory(vec![entry(2, "bag", 1)]);
        inventory.nested.push(([2; 16], source.clone()));
        assert!(matches!(
            plan_output(
                &inventory,
                &source,
                facts(ItemStackClass::NonStackable),
                1,
                &[],
                || Some([3; 16])
            ),
            Err(DromeCubeError::Capacity)
        ));
        inventory.nested.clear();
        if let Some(bag) = inventory.backpack.as_mut() {
            bag.entries = vec![source.clone()];
        }
        assert!(matches!(
            plan_output(
                &inventory,
                &source,
                facts(ItemStackClass::NonStackable),
                1,
                &[],
                || Some([3; 16])
            ),
            Ok(ItemUseFlaskPlan::New { .. })
        ));
    }

    #[test]
    fn item_count_and_ordinal_bounds_fail_without_allocating_an_output() {
        let mut source = entry(1, DROME_CUBE_KEY, 2);
        let before = inventory(vec![source.clone()]);
        assert!(matches!(
            plan_output(
                &before,
                &source,
                facts(ItemStackClass::NonStackable),
                1,
                &[],
                || None
            ),
            Err(DromeCubeError::Capacity)
        ));
        source.placement_ordinal = u64::MAX;
        let inventory = inventory(vec![source.clone()]);
        assert!(matches!(
            plan_output(
                &inventory,
                &source,
                facts(ItemStackClass::NonStackable),
                20,
                &[],
                || None
            ),
            Err(DromeCubeError::ItemUse(ItemUseError::InvalidInput))
        ));
    }

    #[test]
    fn unknown_or_unsupported_stack_classes_fail_closed() {
        let source = entry(1, DROME_CUBE_KEY, 1);
        let inventory = inventory(vec![source.clone()]);
        for stack in [
            ItemStackClass::Unknown,
            ItemStackClass::Stackable {
                proven_maximum: Some(0),
            },
            ItemStackClass::Stackable {
                proven_maximum: Some(101),
            },
        ] {
            assert!(matches!(
                plan_output(&inventory, &source, facts(stack), 20, &[], || Some([3; 16])),
                Err(DromeCubeError::ItemUse(ItemUseError::InvalidInput))
            ));
        }
    }
}
