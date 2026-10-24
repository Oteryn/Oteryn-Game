//! Pure Content facts for pickup definitions; Content identity lookup remains an adapter.

use oteryn_gameplay_domain::domain::item_definition::{
    ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef,
};

/// The Content-binding half of B3-2: no admission, no identity check, no I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupContentError {
    /// No definition in the current Content generation matches the claimed identity.
    DefinitionNotFound,
    /// The identity resolves to a definition that is not an Item.
    NotAnItem,
}

impl std::fmt::Display for PickupContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefinitionNotFound => {
                formatter.write_str("pickup: no Content definition matches the claimed identity")
            }
            Self::NotAnItem => formatter.write_str("pickup: claimed identity is not an Item"),
        }
    }
}

impl std::error::Error for PickupContentError {}

/// The Content adapter's decoded stack capability, independent of its Content carrier.
pub use oteryn_gameplay_content::content::ReferenceItemStackClass as PickupStackClass;

/// Known fields of the exact Item definition resolved by the Content adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickupItemFacts {
    pub weight: Option<u32>,
    pub stack_class: PickupStackClass,
    pub proven_maximum: Option<u32>,
    pub container_capacity: Option<u32>,
    pub container_slot_equip_pattern: bool,
}

/// D82/D83/GAME-ITEM-01 §6.2 facts after the adapter resolves an exact Item identity.
/// Unknown stack classes remain unknown; optional fields are never inferred or defaulted.
pub fn resolve_item_definition_facts(
    definition: &TypedDefinitionRef,
    item: PickupItemFacts,
) -> ItemDefinitionFacts {
    let stack = match item.stack_class {
        PickupStackClass::Unknown => ItemStackClass::Unknown,
        PickupStackClass::NonStackable => ItemStackClass::NonStackable,
        PickupStackClass::StackCapable => ItemStackClass::Stackable {
            proven_maximum: item.proven_maximum,
        },
    };
    ItemDefinitionFacts {
        definition: definition.clone(),
        weight: item.weight,
        stack,
        container_capacity: item.container_capacity,
        container_slot_equip_pattern: item.container_slot_equip_pattern,
    }
}
