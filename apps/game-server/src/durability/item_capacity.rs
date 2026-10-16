//! Conditional B3-3 / BAGS0 §7 weight projection, not item-transfer admission or authority.
//!
//! The transaction owner must prove the direction, complete canonical source/tree and current
//! carried total under its locks, and resolve each exact definition from compatible active
//! Content. `Known` alone does not admit a weight's unit or meaning. Capacity and every weight
//! must already share one admitted integer unit. The separate Reference projection below
//! returns centi-ounces; it does not select the current Character capacity source. B3-3's
//! capacity-source and transaction-integration evidence gates remain open.
//!
//! An incoming container contributes its own weight plus all descendants exactly once. Current
//! carried weight includes the equipment slots and main-backpack tree. Moves within that
//! character do not change carried weight. These input requirements cannot be authenticated
//! by this pure helper. A returned plan must be recomputed on the locked current state before
//! the existing fenced transaction commits; it is neither a reservation nor a durable receipt.

use crate::content::ReferenceItemField;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{GAMEITEM01_STACK_QUANTITY_MAX, InventoryItem};
use serde_json::Value;
use std::collections::BTreeSet;

/// Accepted Formulae revision 1205374 data, shared with `spell_character_facts::maxima`.
/// Packet SHA-256: 78c7a01e329757251a63f0ffa1cb65fdb3682a163e88b2e4dc615a3e74798864.
const REFERENCE_VITALS: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json"
);

/// Reference projection failures, not Character admission or wire refusal codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferenceCapacityRefusal {
    InvalidLevel,
    UnknownVocation,
    InvalidReferenceData,
    ArithmeticOverflow,
}

/// Pure Reference DATA ONLY, in the accepted item-weight unit of centi-ounces (#1336/D207).
/// This is neither current Character authority nor an admitted Character baseline. Formulae
/// gives 400 oz at level 1, while Newhaven gives 600 oz; the existing Character conditional
/// 200 oz and profile/provenance choice remain unresolved (GAME-CHAR-01 Stage-B §9).
/// Do not infer a universal 200 oz adjustment, initialize state, or use this as current
/// transaction capacity without the owning source decision and locked Character facts.
pub(crate) fn project_reference_capacity_centi_oz(
    level: u32,
    vocation: &str,
) -> Result<u64, ReferenceCapacityRefusal> {
    use ReferenceCapacityRefusal::{
        ArithmeticOverflow, InvalidLevel, InvalidReferenceData, UnknownVocation,
    };
    if level == 0 {
        return Err(InvalidLevel);
    }
    let table: Value = serde_json::from_str(REFERENCE_VITALS).map_err(|_| InvalidReferenceData)?;
    if table.get("schema").and_then(Value::as_str) != Some("OTERYN_VOCATION_VITALS_CANDIDATE/v1") {
        return Err(InvalidReferenceData);
    }
    let profiles = table
        .get("vocations")
        .and_then(Value::as_object)
        .ok_or(InvalidReferenceData)?;
    let profile = profiles.get(vocation).ok_or(UnknownVocation)?;
    // Follow maxima's packet-owned rookie boundary and coefficients, without a copied curve.
    let coefficients = || -> Option<(i64, i64)> {
        let formula = if u64::from(level) < profile.get("rookie_until_level")?.as_u64()? {
            "rookie_total"
        } else {
            "total"
        };
        let curve = profile.get("capacity")?.get(formula)?;
        Some((
            curve.get("level_factor")?.as_i64()?,
            curve.get("offset")?.as_i64()?,
        ))
    };
    let (factor, offset) = coefficients().ok_or(InvalidReferenceData)?;
    if factor < 0 {
        return Err(InvalidReferenceData);
    }
    let whole_ounces = factor
        .checked_mul(i64::from(level))
        .and_then(|value| value.checked_add(offset))
        .ok_or(ArithmeticOverflow)?;
    let whole_ounces = u64::try_from(whole_ounces).map_err(|_| InvalidReferenceData)?;
    // Exactly one unit conversion, after the complete whole-ounce reference formula.
    whole_ounces.checked_mul(100).ok_or(ArithmeticOverflow)
}

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
