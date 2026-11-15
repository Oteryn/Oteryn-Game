//! Presentation identity for capability 499's decoded ItemEntry appearance binding.
use oteryn_client_runtime::inventory::ItemEntry;

/// Sprite variants depend on appearance, stack count and fluid sub-type.
/// An absent binding has no atlas key and keeps the missing-image fallback.
pub fn atlas_key(item: &ItemEntry) -> Option<(u16, u32, u32)> {
    (item.appearance_id != 0).then_some((item.appearance_id, item.count.get(), item.sub_type))
}

#[cfg(test)]
#[test]
fn decoded_appearance_count_and_subtype_identify_the_sprite() -> Result<(), &'static str> {
    let mut item = ItemEntry {
        handle: oteryn_client_runtime::inventory::ItemHandle::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
        appearance_id: 0,
    };
    assert_eq!(atlas_key(&item), None);
    item.appearance_id = 3031;
    item.count = std::num::NonZeroU32::new(3).ok_or("nonzero count")?;
    item.sub_type = 7;
    assert_eq!(atlas_key(&item), Some((3031, 3, 7)));
    Ok(())
}
