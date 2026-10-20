//! Immutable item definition facts shared by domain rules and their producers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemForgeProfile {
    pub classification: u8,
    pub max_tier: u8,
}

/// `TypedDefinitionRef = (DefinitionFamily, ProductionKey, DefinitionRevisionRef)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedDefinitionRef {
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}

/// GAME-ITEM-01 §4.1 stack class of a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStackClass {
    NonStackable,
    /// Stackable; `proven_maximum` is the definition's proven stack maximum,
    /// or `None` for the D82 default of 100.
    Stackable {
        proven_maximum: Option<u32>,
    },
    /// Not admitted for pickup (fail closed).
    Unknown,
}

/// Current compatible Content facts for one item definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinitionFacts {
    /// Known physical unit weight in centi-oz from the admitted Content generation.
    pub weight: Option<u32>,
    pub definition: TypedDefinitionRef,
    pub stack: ItemStackClass,
    /// Declared container capacity (direct entries); `None` for a non-container.
    pub container_capacity: Option<u32>,
    /// The definition declares a complete equip pattern whose primary slot is
    /// `container` (GAME-ITEM-01 §6.2). Being a container is not enough.
    pub container_slot_equip_pattern: bool,
}
