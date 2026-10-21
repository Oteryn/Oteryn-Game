//! Starter template and grant outcomes.

use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

/// One starter template: its registered label and revision, and the one item it grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarterTemplate {
    pub key: String,
    pub revision: String,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

impl StarterTemplate {
    /// `oteryn:starter-template.main` (record `oteryn:starter.main_backpack`): one backpack
    /// `oteryn:item.tibia.i2854` into the `container` slot.
    pub fn main() -> Self {
        Self {
            key: "oteryn:starter-template.main".to_owned(),
            revision: "starter-kit-r1".to_owned(),
            definition: TypedDefinitionRef {
                family: "Item".to_owned(),
                production_key: "oteryn:item.tibia.i2854".to_owned(),
                revision_ref: "definition-r1".to_owned(),
            },
            quantity: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterGrantOutcome {
    /// The fresh main backpack.
    Minted([u8; 16]),
    Skipped,
    AlreadyGranted,
    NotAdmitted,
}
