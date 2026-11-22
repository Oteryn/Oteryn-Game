//! Re-encode session-owned inventory with the capture's negotiated wire profile.
use super::TestResult;
use oteryn_protocol_oteryn::item_view::{self, CharacterInventory, ItemViewWireError};

fn profile(selected: &[u32]) -> (bool, bool) {
    (
        selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
        selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    )
}

fn encode_inventory(
    selected: &[u32],
    inventory: &CharacterInventory,
) -> Result<Vec<u8>, ItemViewWireError> {
    let (equip_drop, appearances) = profile(selected);
    item_view::encode_character_inventory_with_appearances(inventory, equip_drop, appearances)
}

pub(super) fn encode(
    scenario: &str,
    selected: &[u32],
    inventory: &CharacterInventory,
) -> TestResult<Vec<u8>> {
    let payload = encode_inventory(selected, inventory)
        .map_err(|error| format!("{scenario} inventory capture: {error:?}"))?;
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let (equip_drop, appearances) = profile(selected);
        std::fs::write(
            std::path::Path::new(&root).join(format!("{scenario}-inventory-profile.txt")),
            format!(
                "equip_drop={}\nappearances={}\n",
                u8::from(equip_drop),
                u8::from(appearances)
            ),
        )?;
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use item_view::{EquipmentSlot, EquippedItem, ItemEntry};
    use std::num::{NonZeroU32, NonZeroU64};

    fn entry(handle: u64, appearance_id: u16) -> TestResult<ItemEntry> {
        Ok(ItemEntry {
            handle: NonZeroU64::new(handle).ok_or("handle")?,
            item_definition_ref: NonZeroU32::new(1000 + u32::from(appearance_id))
                .ok_or("definition")?,
            count: NonZeroU32::new(3).ok_or("count")?,
            sub_type: 7,
            appearance_id,
        })
    }

    #[test]
    fn captured_inventory_preserves_negotiated_equipment() -> TestResult {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(1, 0)?),
            entries: vec![entry(2, 0)?],
            equipment: vec![EquippedItem {
                slot: EquipmentSlot::Head,
                item: entry(3, 0)?,
            }],
        };
        let payload = encode_inventory(&[4, 6, 12], &inventory)
            .map_err(|error| format!("inventory capture: {error:?}"))?;
        let decoded = item_view::decode_character_inventory_with_appearances(&payload, true, false)
            .map_err(|error| format!("inventory replay: {error:?}"))?;
        assert_eq!(decoded, inventory);
        assert!(item_view::decode_character_inventory(&payload).is_err());
        Ok(())
    }

    #[test]
    fn captured_inventory_preserves_negotiated_appearances() -> TestResult {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(1, 731)?),
            entries: vec![entry(2, 732)?],
            equipment: Vec::new(),
        };
        let payload = encode_inventory(&[4, 6, 499], &inventory)
            .map_err(|error| format!("inventory capture: {error:?}"))?;
        let decoded = item_view::decode_character_inventory_with_appearances(&payload, false, true)
            .map_err(|error| format!("inventory replay: {error:?}"))?;
        assert_eq!(decoded, inventory);
        assert!(item_view::decode_character_inventory(&payload).is_err());
        Ok(())
    }

    #[test]
    fn inventory_capture_preserves_unselected_and_malformed_refusals() -> TestResult {
        let item = entry(1, 0)?;
        let mut inventory = CharacterInventory {
            main_backpack: None,
            entries: Vec::new(),
            equipment: vec![EquippedItem {
                slot: EquipmentSlot::Head,
                item,
            }],
        };
        assert!(encode_inventory(&[4, 6], &inventory).is_err());
        inventory.main_backpack = Some(item);
        assert!(encode_inventory(&[4, 6, 12, 499], &inventory).is_err());
        let legacy = CharacterInventory {
            main_backpack: Some(entry(1, 0)?),
            entries: vec![entry(2, 0)?],
            equipment: Vec::new(),
        };
        assert_eq!(
            encode_inventory(&[4, 6], &legacy),
            item_view::encode_character_inventory(&legacy)
        );
        Ok(())
    }
}
