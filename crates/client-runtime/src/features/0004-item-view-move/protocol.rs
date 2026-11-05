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
