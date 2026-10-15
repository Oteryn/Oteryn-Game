//! HOUSE-RUNTIME-1a immutable house-side audit validation.
//!
//! This deliberately has no protobuf tags, envelope encoder or decoder: the CP must
//! lease/register the new operation and qualify its worst-case bound in the slot.
//! Existing OneItemTransferV1 and type-2 validation remain unchanged. Passing this
//! validator does not prove a committed shape or independently current authority.

use super::house_interior::{HouseInteriorEffect, HouseInteriorError};
use crate::domain::ItemInstanceId;
use crate::foundation::CharacterId;

/// House-side facts to be carried by the registered one-item operation, together
/// with the character-side location, item state, command and normal envelope facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseInteriorAuditChange {
    pub item_instance_id: ItemInstanceId,
    pub actor: CharacterId,
    pub effect: HouseInteriorEffect,
}

impl HouseInteriorAuditChange {
    pub fn validate(&self) -> Result<(), HouseInteriorError> {
        self.effect.validate(self.item_instance_id, self.actor)
    }
}
