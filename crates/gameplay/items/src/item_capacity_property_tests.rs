use crate::item_capacity::{
    CapacityMove, CapacityPlan, CapacityRefusal, IncomingWeight, plan_capacity,
};
use crate::item_transfer::{
    BackpackEntry, InventoryItem, ItemTransferDestination, ItemTransferRefusal, PlanInput,
    TransferPlan, plan_transfer,
};
use oteryn_gameplay_domain::domain::item_definition::{
    ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef,
};
use oteryn_test_support::property::for_each_case;

fn facts(key: &str, capacity: Option<u32>) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "r1".into(),
        },
        weight: Some(1),
        stack: ItemStackClass::Stackable {
            proven_maximum: Some(100),
        },
        container_capacity: capacity,
        container_slot_equip_pattern: capacity.is_some(),
    }
}

fn item(facts: &ItemDefinitionFacts, id: u8, quantity: u32) -> InventoryItem {
    InventoryItem {
        item_instance_id: [id; 16],
        definition: facts.definition.clone(),
        quantity,
    }
}

#[test]
fn property_container_admission_never_exceeds_slots_or_partially_tops_up_on_refusal() {
    for_each_case("container limits", |rng, _| {
        let capacity = *rng.pick(&[0, 1, 2, 20, 21, u32::MAX]);
        // Owners supply a valid before-state; concentrate on empty/full and
        // partial bags within the declared limit rather than invalid snapshots.
        let count = rng.below(u64::from(capacity.clamp(1, 20)) + 1) as usize;
        let source_facts = facts("source", None);
        let backpack_facts = facts("bag", Some(capacity));
        let source_quantity = 1 + rng.below(100) as u32;
        let receiver_quantity = 1 + rng.below(100) as u32;
        let source = item(&source_facts, 1, source_quantity);
        let bag = item(&backpack_facts, 2, 1);
        let entries: Vec<_> = (0..count)
            .map(|i| BackpackEntry {
                item: item(&source_facts, i as u8 + 3, receiver_quantity),
                placement_ordinal: i as u64 + 1,
            })
            .collect();
        let input = PlanInput {
            destination: ItemTransferDestination::MainBackpack,
            item: &source_facts,
            backpack: Some(&backpack_facts),
            source: &source,
            source_has_entries: false,
            slot: Some(&bag),
            entries: &entries,
        };
        let plan = plan_transfer(&input);
        if !(1..=20).contains(&capacity) {
            assert_eq!(plan, Err(ItemTransferRefusal::UnsupportedContainerCapacity));
        } else {
            let full_merge = count > 0 && source_quantity <= 100 - receiver_quantity;
            assert_eq!(plan.is_ok(), count < capacity as usize || full_merge);
            match plan {
                Ok(TransferPlan::FullMerge { receiver, .. }) => {
                    assert!(receiver.quantity_after <= 100);
                    assert_eq!(
                        receiver.quantity_after - receiver.quantity_before,
                        source_quantity
                    );
                    assert_eq!(receiver.item_instance_id, [(count + 2) as u8; 16]);
                }
                Ok(TransferPlan::TopUp {
                    receiver,
                    source_after,
                    ..
                }) => {
                    assert!(count < capacity as usize);
                    assert_eq!(receiver.quantity_after, 100);
                    assert_eq!(
                        receiver.quantity_after - receiver.quantity_before + source_after,
                        source_quantity
                    );
                }
                Ok(TransferPlan::NewEntry { .. }) => assert!(count < capacity as usize),
                other => assert_eq!(other, Err(ItemTransferRefusal::MainBackpackFull)),
            }
        }
        let equip = PlanInput {
            destination: ItemTransferDestination::ContainerSlot,
            item: &backpack_facts,
            source: &bag,
            slot: None,
            ..input
        };
        assert_eq!(
            plan_transfer(&equip),
            if (1..=20).contains(&capacity) {
                Ok(TransferPlan::ContainerSlot)
            } else {
                Err(ItemTransferRefusal::UnsupportedContainerCapacity)
            }
        );
        Ok(())
    });
}

#[test]
fn property_carry_capacity_uses_checked_totals_and_rejects_instead_of_clamping() {
    for_each_case("carry capacity", |rng, case| {
        let facts = facts("incoming", None);
        let items: Vec<_> = (0..=case % 8)
            .map(|index| {
                (
                    item(&facts, index as u8 + 1, 1 + rng.below(100) as u32),
                    rng.u64() as u32,
                )
            })
            .collect();
        let incoming: Vec<_> = items
            .iter()
            .map(|(item, weight)| IncomingWeight {
                item,
                weight_definition: &facts.definition,
                unit_weight: Some(*weight),
            })
            .collect();
        let added: u64 = items
            .iter()
            .map(|(item, weight)| u64::from(item.quantity) * u64::from(*weight))
            .sum();
        for before in [0, rng.u64(), u64::MAX] {
            for capacity in [0, added, rng.u64(), u64::MAX] {
                let result = plan_capacity(CapacityMove::IntoCharacter {
                    carried_weight: Some(before),
                    capacity: Some(capacity),
                    incoming: &incoming,
                });
                let total = u128::from(before) + u128::from(added);
                let expected = if total > u128::from(u64::MAX) {
                    Err(CapacityRefusal::ArithmeticOverflow)
                } else if total > u128::from(capacity) {
                    Err(CapacityRefusal::Overweight {
                        required: total as u64,
                        capacity,
                    })
                } else {
                    Ok(CapacityPlan::Incoming {
                        weight_before: before,
                        added_weight: added,
                        weight_after: total as u64,
                        remaining_capacity: capacity - total as u64,
                    })
                };
                assert_eq!(result, expected);
            }
        }
        Ok(())
    });
}

#[test]
fn property_incoming_tree_bound_is_inclusive_and_quantities_fail_closed() {
    for_each_case("incoming tree limits", |rng, _| {
        let facts = facts("incoming", None);
        let count = *rng.pick(&[0, 1, 499, 500, 501]);
        let items: Vec<_> = (0..count)
            .map(|i: u128| InventoryItem {
                item_instance_id: i.to_le_bytes(),
                definition: facts.definition.clone(),
                quantity: 1,
            })
            .collect();
        let incoming: Vec<_> = items
            .iter()
            .map(|item| IncomingWeight {
                item,
                weight_definition: &facts.definition,
                unit_weight: Some(0),
            })
            .collect();
        let result = plan_capacity(CapacityMove::IntoCharacter {
            carried_weight: Some(0),
            capacity: Some(0),
            incoming: &incoming,
        });
        match count {
            0 => assert_eq!(result, Err(CapacityRefusal::EmptyIncoming)),
            501 => assert_eq!(result, Err(CapacityRefusal::TooManyItems)),
            _ => assert!(result.is_ok()),
        }
        for quantity in [0, 101, u32::MAX] {
            let source = item(&facts, 1, quantity);
            let incoming = [IncomingWeight {
                item: &source,
                weight_definition: &facts.definition,
                unit_weight: Some(0),
            }];
            assert_eq!(
                plan_capacity(CapacityMove::IntoCharacter {
                    carried_weight: Some(0),
                    capacity: Some(u64::MAX),
                    incoming: &incoming,
                }),
                Err(CapacityRefusal::InvalidQuantity)
            );
        }
        Ok(())
    });
}
