//! Live inventory projection; handles retain their session identity.
pub use oteryn_session::{CharacterInventory, ItemEntry, ItemHandle};

#[must_use]
pub fn initial<S: oteryn_session::SessionStream>(
    session: &oteryn_session::Session<S>,
) -> Option<CharacterInventory> {
    session.inventory().cloned()
}

/// Equipment and container trees have separate panels.
#[must_use]
pub fn rows(inventory: &CharacterInventory) -> Vec<ItemEntry> {
    inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .copied()
        .collect()
}

pub use oteryn_session::item_movement::{
    MoveRequest, MoveResponse, RefusalDetail, WorldTilePosition,
};
pub use oteryn_session::{ItemMoveDestination, ItemMoveOutcome};

pub async fn send<S: oteryn_session::SessionStream>(
    session: &mut oteryn_session::Session<S>,
    request: MoveRequest,
) -> Result<Option<MoveResponse>, oteryn_session::SessionError> {
    if !crate::equipment::available(request.destination, session.is_selected(12))
        || !session.is_selected(4)
        || (request.count.is_some() && !session.is_selected(61))
        || (matches!(request.destination, ItemMoveDestination::Container(_))
            && !session.is_selected(14))
        || (matches!(request.destination, ItemMoveDestination::Ground(_))
            && !session.is_selected(12)
            && !session.is_selected(11))
    {
        return Ok(None);
    }
    session.move_item_counted(request).await.map(Some)
}
pub fn feedback(response: Option<MoveResponse>) -> &'static str {
    let Some(response) = response else {
        return "Item movement unavailable for this session";
    };
    if let Some(detail) = response.detail {
        return detail.reference_message();
    }
    match response.outcome {
        ItemMoveOutcome::Moved => "Item moved",
        ItemMoveOutcome::Stale => "Item is no longer available",
        ItemMoveOutcome::TooFar => "Too far away",
        ItemMoveOutcome::NoBackpack => "No backpack",
        ItemMoveOutcome::NoRoom => "There is not enough room",
        ItemMoveOutcome::NotOwner => "You do not own this item",
        ItemMoveOutcome::NotPickupable => "You cannot take this object",
        ItemMoveOutcome::NotSupported => "This item cannot be moved there",
        ItemMoveOutcome::Rejected => "Item move refused",
        ItemMoveOutcome::SlotMismatch => "This item does not fit this slot",
        ItemMoveOutcome::RequirementNotMet => "You do not meet this item’s requirements",
        ItemMoveOutcome::Blocked => "Cannot move this item there",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_preserves_server_display_order() -> Result<(), Box<dyn std::error::Error>> {
        let entry = |handle: u64| -> Result<ItemEntry, Box<dyn std::error::Error>> {
            Ok(ItemEntry {
                handle: handle.try_into()?,
                item_definition_ref: 42.try_into()?,
                count: 1.try_into()?,
                sub_type: 0,
            })
        };
        let backpack = entry(4)?;
        let first = entry(8)?;
        let second = entry(2)?;
        let inventory = CharacterInventory {
            main_backpack: Some(backpack),
            entries: vec![first, second],
            equipment: vec![],
        };
        assert_eq!(rows(&inventory), vec![backpack, first, second]);
        assert!(rows(&CharacterInventory::default()).is_empty());
        Ok(())
    }
}
