//! IMBUE-FORGE-0 §§3–7,15: raw slot state and structural checks, not admission authority.
//! No catalogue/quest source is admitted here. Native Premium is only a subcheck; current
//! account/session/lease/scope and admitted source facts remain independently owner-resolved.

use super::ItemInstanceId;
use super::premium::{PremiumGate, PremiumRefusal};
use std::collections::BTreeSet;

pub(crate) const MAX_SLOTS: usize = 3;
pub(crate) const MAX_REMAINING_MS: u64 = 72_000_000;

/// Accepted tier names, not protocol IDs or a conversion from the incompatible item codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tier {
    Basic,
    Intricate,
    Powerful,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyRoute {
    Shrine,
    ScrollCreate,
    ScrollApply,
}

/// Every successful subcheck still needs these unresolved facts. No variant grants access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutstandingAccess {
    QuestAndContentBindings,
    ItemAndContentBindings,
}

pub(crate) fn outstanding_access(
    route: ApplyRoute,
    tier: Tier,
    premium: PremiumGate,
) -> Result<OutstandingAccess, PremiumRefusal> {
    if route == ApplyRoute::ScrollCreate || (route == ApplyRoute::Shrine && tier != Tier::Basic) {
        premium.require()?;
    }
    Ok(if route == ApplyRoute::ScrollApply {
        OutstandingAccess::ItemAndContentBindings
    } else {
        OutstandingAccess::QuestAndContentBindings
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateError {
    InvalidBinding,
    InvalidRemaining,
    UnresolvedItemRules,
    InvalidItemRules,
    SlotUnavailable,
    CategoryOccupied,
    TierNotAllowed,
    InvalidSnapshot,
}

/// Raw persisted/request binding data. Nonempty refs are not catalogue-admission evidence.
/// `content_revision` is the applying definition revision, never a timed write counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotBinding {
    pub(crate) item: ItemInstanceId,
    pub(crate) slot: u8,
    pub(crate) key: String,
    pub(crate) content_revision: String,
    pub(crate) category: String,
    pub(crate) tier: Tier,
    pub(crate) applying_occurrence: [u8; 16],
    /// The applying transaction; a later checkpoint does not replace its identity.
    pub(crate) transaction_id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotRecord {
    binding: SlotBinding,
    remaining_ms: u64,
}

fn reference_present(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

impl SlotRecord {
    pub(crate) fn new(binding: SlotBinding, remaining_ms: u64) -> Result<Self, StateError> {
        if usize::from(binding.slot) >= MAX_SLOTS
            || !reference_present(&binding.key)
            || !reference_present(&binding.content_revision)
            || !reference_present(&binding.category)
            || super::validate_uuid_v7(&binding.applying_occurrence).is_err()
            || super::validate_uuid_v7(&binding.transaction_id).is_err()
        {
            return Err(StateError::InvalidBinding);
        }
        if remaining_ms > MAX_REMAINING_MS {
            return Err(StateError::InvalidRemaining);
        }
        Ok(Self {
            binding,
            remaining_ms,
        })
    }

    pub(crate) fn binding(&self) -> &SlotBinding {
        &self.binding
    }

    pub(crate) fn remaining_ms(&self) -> u64 {
        self.remaining_ms
    }
}

/// Structural item rules only; this never proves the item/category/rules came from admitted
/// content or a current holder. Unknown rules refuse, including completed-scroll use.
pub(crate) fn check_target_slot(
    item: ItemInstanceId,
    rows: &[SlotRecord],
    declared_slots: Option<u8>,
    allowed_maximum: Option<Tier>,
    slot: u8,
    category: &str,
    tier: Tier,
) -> Result<(), StateError> {
    let (Some(slots), Some(maximum)) = (declared_slots, allowed_maximum) else {
        return Err(StateError::UnresolvedItemRules);
    };
    if usize::from(slots) > MAX_SLOTS || !reference_present(category) {
        return Err(StateError::InvalidItemRules);
    }
    if rows.len() > MAX_SLOTS {
        return Err(StateError::InvalidSnapshot);
    }
    let mut occupied = BTreeSet::new();
    let mut categories = BTreeSet::new();
    for row in rows {
        let binding = row.binding();
        if binding.item != item
            || !occupied.insert(binding.slot)
            || !categories.insert(binding.category.as_str())
        {
            return Err(StateError::InvalidSnapshot);
        }
    }
    if slot >= slots || occupied.contains(&slot) {
        return Err(StateError::SlotUnavailable);
    }
    // A zero-time row remains present/occupied until the audited expiry deletes it.
    if categories.contains(category) {
        return Err(StateError::CategoryOccupied);
    }
    if tier > maximum {
        return Err(StateError::TierNotAllowed);
    }
    Ok(())
}
