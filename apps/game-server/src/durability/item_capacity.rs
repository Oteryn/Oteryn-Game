//! Conditional B3-3 / BAGS0 §7 weight projection, not item-transfer admission or authority.
//!
//! The transaction owner must prove the direction, complete canonical source/tree and current
//! carried total under its locks, and resolve each exact definition from compatible active
//! Content. `Known` alone does not admit a weight's unit or meaning. Capacity and every weight
//! must already share one admitted integer unit; no scale, conversion or capacity formula is
//! supplied here. B3-3's weight/capacity-source evidence gates remain open.
//!
//! An incoming container contributes its own weight plus all descendants exactly once. Current
//! carried weight includes the equipment slots and main-backpack tree. Moves within that
//! character do not change carried weight. These input requirements cannot be authenticated
//! by this pure helper. A returned plan must be recomputed on the locked current state before
//! the existing fenced transaction commits; it is neither a reservation nor a durable receipt.

use crate::content::ReferenceItemField;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{GAMEITEM01_STACK_QUANTITY_MAX, InventoryItem};
use std::collections::BTreeSet;

/// BAGS0-RL-02: one incoming container tree, root included; no truncation.
const INCOMING_TREE_ITEMS_MAX: usize = 500;

/// One item of the owner's already qualified canonical incoming source/tree.
#[derive(Clone, Copy)]
pub(crate) struct IncomingWeight<'a> {
    pub item: &'a InventoryItem,
    /// Exact family, production key and revision for the supplied per-unit weight.
    pub weight_definition: &'a TypedDefinitionRef,
    pub unit_weight: &'a ReferenceItemField<u32>,
}

/// Direction is a current transaction-owner fact, never a client assertion. This helper does
/// not prove ownership, containment, source liveness or completeness of the incoming tree.
pub(crate) enum CapacityMove<'a> {
    WithinCharacter,
    IntoCharacter {
        carried_weight: Option<u64>,
        capacity: Option<u64>,
        incoming: &'a [IncomingWeight<'a>],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapacityPlan {
    Unchanged,
    Incoming {
        weight_before: u64,
        added_weight: u64,
        weight_after: u64,
        remaining_capacity: u64,
    },
}

/// Local preparation errors, not new wire or registry refusal codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapacityRefusal {
    UnknownCarriedWeight,
    UnknownCapacity,
    EmptyIncoming,
    TooManyItems,
    DuplicateItem,
    DefinitionMismatch,
    InvalidQuantity,
    UnknownItemWeight,
    ArithmeticOverflow,
    Overweight { required: u64, capacity: u64 },
}

/// Reuse the already admitted placement/stack plan; this adds only weight accounting. It does
/// not choose stack receivers, enforce definition-specific stack limits, validate container
/// graphs, infer ignored descendants or clamp a requested pickup to the available capacity.
pub(crate) fn plan_capacity(movement: CapacityMove<'_>) -> Result<CapacityPlan, CapacityRefusal> {
    let CapacityMove::IntoCharacter {
        carried_weight,
        capacity,
        incoming,
    } = movement
    else {
        return Ok(CapacityPlan::Unchanged);
    };
    let weight_before = carried_weight.ok_or(CapacityRefusal::UnknownCarriedWeight)?;
    let capacity = capacity.ok_or(CapacityRefusal::UnknownCapacity)?;
    if incoming.is_empty() {
        return Err(CapacityRefusal::EmptyIncoming);
    }
    if incoming.len() > INCOMING_TREE_ITEMS_MAX {
        return Err(CapacityRefusal::TooManyItems);
    }
    let mut identities = BTreeSet::new();
    let mut added_weight = 0_u64;
    for row in incoming {
        if !identities.insert(row.item.item_instance_id) {
            return Err(CapacityRefusal::DuplicateItem);
        }
        if row.item.definition != *row.weight_definition {
            return Err(CapacityRefusal::DefinitionMismatch);
        }
        if !(1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&row.item.quantity) {
            return Err(CapacityRefusal::InvalidQuantity);
        }
        let ReferenceItemField::Known(weight) = row.unit_weight else {
            return Err(CapacityRefusal::UnknownItemWeight);
        };
        let item_weight = u64::from(*weight)
            .checked_mul(u64::from(row.item.quantity))
            .ok_or(CapacityRefusal::ArithmeticOverflow)?;
        added_weight = added_weight
            .checked_add(item_weight)
            .ok_or(CapacityRefusal::ArithmeticOverflow)?;
    }
    let weight_after = weight_before
        .checked_add(added_weight)
        .ok_or(CapacityRefusal::ArithmeticOverflow)?;
    let remaining_capacity =
        capacity
            .checked_sub(weight_after)
            .ok_or(CapacityRefusal::Overweight {
                required: weight_after,
                capacity,
            })?;
    Ok(CapacityPlan::Incoming {
        weight_before,
        added_weight,
        weight_after,
        remaining_capacity,
    })
}
