#![allow(clippy::unwrap_used)]

// Actual native types; only the allocated, unregistered capacity planner is path-loaded.
use content::ReferenceItemField::{self, Conflict, Known, NotApplicable, Unknown};
use durability::item_mint::TypedDefinitionRef;
use durability::item_transfer::InventoryItem;
use oteryn_game_server::{content, durability};

#[path = "../src/durability/item_capacity.rs"]
mod item_capacity;

use item_capacity::{CapacityMove, CapacityPlan, CapacityRefusal, IncomingWeight, plan_capacity};

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item/{key}"),
        revision_ref: "r1".into(),
    }
}

fn item(tag: u16, quantity: u32) -> InventoryItem {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[14..].copy_from_slice(&tag.to_be_bytes());
    InventoryItem {
        item_instance_id: id,
        definition: definition("synthetic"),
        quantity,
    }
}

fn weighted<'a>(
    item: &'a InventoryItem,
    weight: &'a ReferenceItemField<u32>,
) -> IncomingWeight<'a> {
    IncomingWeight {
        item,
        weight_definition: &item.definition,
        unit_weight: weight,
    }
}

fn incoming(
    carried: Option<u64>,
    capacity: Option<u64>,
    items: &[IncomingWeight<'_>],
) -> Result<CapacityPlan, CapacityRefusal> {
    plan_capacity(CapacityMove::IntoCharacter {
        carried_weight: carried,
        capacity,
        incoming: items,
    })
}

#[test]
fn exact_capacity_fits_and_one_unit_over_refuses_without_mutating_input() {
    let source = item(1, 2);
    let rows = [weighted(&source, &Known(3))];
    assert_eq!(
        incoming(Some(4), Some(10), &rows),
        Ok(CapacityPlan::Incoming {
            weight_before: 4,
            added_weight: 6,
            weight_after: 10,
            remaining_capacity: 0,
        })
    );
    assert_eq!(
        incoming(Some(5), Some(10), &rows),
        Err(CapacityRefusal::Overweight {
            required: 11,
            capacity: 10
        })
    );
    assert_eq!(source, item(1, 2));
}

#[test]
fn an_external_container_counts_its_own_weight_and_every_supplied_descendant_once() {
    // Synthetic canonical flattening supplied by the future owner, not a proven live graph.
    let root = item(1, 1);
    let nested_bag = item(2, 1);
    let stack = item(3, 7);
    let rows = [
        weighted(&root, &Known(5)),
        weighted(&nested_bag, &Known(3)),
        weighted(&stack, &Known(2)),
    ];
    assert_eq!(
        incoming(Some(8), Some(30), &rows),
        Ok(CapacityPlan::Incoming {
            weight_before: 8,
            added_weight: 22,
            weight_after: 30,
            remaining_capacity: 0,
        })
    );
    let reversed = [rows[2], rows[1], rows[0]];
    assert_eq!(
        incoming(Some(8), Some(30), &rows),
        incoming(Some(8), Some(30), &reversed)
    );
}

#[test]
fn internal_character_moves_do_not_charge_weight_or_require_a_capacity_read() {
    // Classification must be proven by the future owner; this variant is not client authority.
    assert_eq!(
        plan_capacity(CapacityMove::WithinCharacter),
        Ok(CapacityPlan::Unchanged)
    );
}

#[test]
fn merging_a_ground_stack_still_adds_all_incoming_units() {
    let source = item(1, 100);
    assert_eq!(
        incoming(Some(1), Some(100), &[weighted(&source, &Known(1))]),
        Err(CapacityRefusal::Overweight {
            required: 101,
            capacity: 100
        })
    );
    // This planner does not select a receiver or duplicate the existing merge/entry planner.
}

#[test]
fn unknown_current_weight_or_capacity_is_never_zero_or_unlimited() {
    let source = item(1, 1);
    let rows = [weighted(&source, &Known(0))];
    assert_eq!(
        incoming(None, Some(0), &rows),
        Err(CapacityRefusal::UnknownCarriedWeight)
    );
    assert_eq!(
        incoming(Some(0), None, &rows),
        Err(CapacityRefusal::UnknownCapacity)
    );
    assert!(incoming(Some(0), Some(0), &rows).is_ok());
}

#[test]
fn unknown_and_ambiguous_item_weights_fail_closed() {
    let source = item(1, 1);
    for weight in [Unknown, Conflict, NotApplicable] {
        assert_eq!(
            incoming(Some(0), Some(100), &[weighted(&source, &weight)]),
            Err(CapacityRefusal::UnknownItemWeight)
        );
    }
}

#[test]
fn weight_facts_must_match_the_entire_native_definition_reference() {
    let source = item(1, 1);
    for mismatch in [
        TypedDefinitionRef {
            family: "Creature".into(),
            ..source.definition.clone()
        },
        definition("another"),
        TypedDefinitionRef {
            revision_ref: "r2".into(),
            ..source.definition.clone()
        },
    ] {
        let row = IncomingWeight {
            item: &source,
            weight_definition: &mismatch,
            unit_weight: &Known(1),
        };
        assert_eq!(
            incoming(Some(0), Some(100), &[row]),
            Err(CapacityRefusal::DefinitionMismatch)
        );
    }
}

#[test]
fn invalid_quantities_and_duplicate_instances_are_refused() {
    for quantity in [0, 101, u32::MAX] {
        let source = item(1, quantity);
        assert_eq!(
            incoming(Some(0), Some(u64::MAX), &[weighted(&source, &Known(1))]),
            Err(CapacityRefusal::InvalidQuantity)
        );
    }
    let source = item(1, 1);
    let row = weighted(&source, &Known(1));
    assert_eq!(
        incoming(Some(0), Some(100), &[row, row]),
        Err(CapacityRefusal::DuplicateItem)
    );
}

#[test]
fn checked_totals_do_not_wrap_or_saturate_to_fit() {
    let source = item(1, 1);
    assert_eq!(
        incoming(
            Some(u64::MAX),
            Some(u64::MAX),
            &[weighted(&source, &Known(1))]
        ),
        Err(CapacityRefusal::ArithmeticOverflow)
    );
    assert_eq!(
        incoming(
            Some(u64::MAX - 1),
            Some(u64::MAX),
            &[weighted(&source, &Known(1))]
        ),
        Ok(CapacityPlan::Incoming {
            weight_before: u64::MAX - 1,
            added_weight: 1,
            weight_after: u64::MAX,
            remaining_capacity: 0
        })
    );
}

#[test]
fn full_width_weight_and_stack_count_are_exact_instead_of_u32_wrapping() {
    let source = item(1, 100);
    let added = u64::from(u32::MAX) * 100;
    assert_eq!(
        incoming(Some(0), Some(added), &[weighted(&source, &Known(u32::MAX))]),
        Ok(CapacityPlan::Incoming {
            weight_before: 0,
            added_weight: added,
            weight_after: added,
            remaining_capacity: 0
        })
    );
}

#[test]
fn empty_or_over_bound_incoming_tree_is_not_silently_truncated() {
    assert_eq!(
        incoming(Some(0), Some(0), &[]),
        Err(CapacityRefusal::EmptyIncoming)
    );
    let items: Vec<_> = (0_u16..501).map(|tag| item(tag, 1)).collect();
    let rows: Vec<_> = items
        .iter()
        .map(|source| weighted(source, &Known(0)))
        .collect();
    assert!(incoming(Some(0), Some(0), &rows[..500]).is_ok());
    assert_eq!(
        incoming(Some(0), Some(0), &rows),
        Err(CapacityRefusal::TooManyItems)
    );
}

#[test]
fn a_second_request_must_be_replanned_against_the_updated_carried_total() {
    let first = item(1, 1);
    let second = item(2, 1);
    assert!(incoming(Some(4), Some(10), &[weighted(&first, &Known(4))]).is_ok());
    assert!(incoming(Some(4), Some(10), &[weighted(&second, &Known(4))]).is_ok());
    assert_eq!(
        incoming(Some(8), Some(10), &[weighted(&second, &Known(4))]),
        Err(CapacityRefusal::Overweight {
            required: 12,
            capacity: 10
        })
    );
    // Pure inputs only: the future transaction lock/reread must actually serialize requests.
}
