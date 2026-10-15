//! PLAYER_TRADE0 leaf policy and native offer data, not admission or a reservation proof.
use crate::content::{ReferenceItemField, ReferenceItemTradeRestrictions};
use crate::domain::AccountId;
use crate::durability::item_mint_audit::{ITEM_LIFECYCLE_LIVE, OneItemStateV1};
use crate::durability::item_timed_state::StoredTimedState;
use crate::durability::item_transfer::{
    CharacterBackpack, ContainerEntryPosition, CurrentCharacterItemFence,
    GAMEITEM01_STACK_QUANTITY_MAX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Trader {
    pub account_id: AccountId,
    pub fence: CurrentCharacterItemFence,
}

/// Actual loaded native instance state and direct-entry position. The owner must also qualify
/// pickupability, non-corpse/empty-container classification, active definitions and ownership.
/// `timed: None` means a proven absent timed row, never an unavailable read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafOffer {
    pub state: OneItemStateV1,
    pub timed: Option<StoredTimedState>,
    pub position: ContainerEntryPosition,
}

impl LeafOffer {
    pub(crate) fn matches_backpack(&self, backpack: &CharacterBackpack, world: &[u8; 16]) -> bool {
        self.state.lifecycle == ITEM_LIFECYCLE_LIVE
            && self.state.world_id.as_slice() == world.as_slice()
            && (1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&self.state.quantity)
            && self.position.parent_item_instance_id == backpack.backpack.item_instance_id
            && self.state.item_instance_id.as_slice()
                != backpack.backpack.item_instance_id.as_slice()
            && backpack.entries.iter().any(|entry| {
                entry.placement_ordinal == self.position.placement_ordinal
                    && entry.item.item_instance_id.as_slice()
                        == self.state.item_instance_id.as_slice()
                    && entry.item.quantity == self.state.quantity
                    && self.state.definition.as_ref().is_some_and(|definition| {
                        definition.family == entry.item.definition.family
                            && definition.production_key == entry.item.definition.production_key
                            && definition.revision_ref == entry.item.definition.revision_ref
                    })
            })
    }
}

/// Unknown direct-trade permission is explicitly allowed; marketable is not a substitute.
/// This checks only the flag, not the other offer/current-authority requirements.
pub(crate) fn tradeable_flag_allows(restrictions: &ReferenceItemTradeRestrictions) -> bool {
    matches!(
        restrictions.tradeable,
        ReferenceItemField::Unknown | ReferenceItemField::Known(true)
    )
}
