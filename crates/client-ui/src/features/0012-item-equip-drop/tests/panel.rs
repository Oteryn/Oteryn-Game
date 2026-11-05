use super::*;

#[test]
fn slots_project_live_identity_and_clear_on_delta_or_disconnect()
-> Result<(), Box<dyn std::error::Error>> {
    let item = ItemEntry {
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 3,
    };
    let mut inventory = CharacterInventory {
        main_backpack: Some(item),
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Head,
            item,
        }],
        ..Default::default()
    };
    let slots = rows(Some(&inventory), true);
    assert_eq!(slots.len(), 10);
    assert_eq!(slots[0].item, Some(item));
    assert_eq!(
        slots[0].destination,
        ItemMoveDestination::Equipment(EquipmentSlot::Head)
    );
    assert_eq!(slots[2].destination, ItemMoveDestination::MainBackpack);
    assert_eq!(slots[2].item, Some(item));
    assert_eq!(slots[1].text(), "Neck: empty");
    assert_eq!(slots[0].drag_item().map(|i| i.handle), Some(item.handle));
    inventory.equipment.clear();
    assert_eq!(rows(Some(&inventory), true)[0].drag_item(), None);
    assert!(rows(None, true).iter().all(|s| s.item.is_none()));
    assert_eq!(rows(None, true)[0].text(), "Head: unavailable");
    Ok(())
}

#[test]
fn equip_unequip_and_drop_keep_the_handle_and_wait_for_server()
-> Result<(), Box<dyn std::error::Error>> {
    let item = crate::inventory::movement::DragItem {
        equipped: false,
        handle: 7.try_into()?,
        count: 1,
    };
    for destination in [
        ItemMoveDestination::Equipment(EquipmentSlot::RightHand),
        ItemMoveDestination::MainBackpack,
        ItemMoveDestination::Container(8.try_into()?),
        ItemMoveDestination::Ground(WorldTilePosition {
            x: 2,
            y: 3,
            floor: 0,
        }),
    ] {
        let mut movement = crate::inventory::movement::Movement::default();
        assert!(movement.begin(item));
        let request = movement.drop_on(Some(destination), true).ok_or("request")?;
        assert_eq!(request.source, item.handle);
        assert_eq!(request.destination, destination);
        assert!(movement.pending);
        assert!(!movement.begin(item));
    }
    Ok(())
}

#[test]
fn equip_ammo_sends_the_whole_stack_without_a_count_extension()
-> Result<(), Box<dyn std::error::Error>> {
    let mut movement = crate::inventory::movement::Movement::default();
    movement.begin(DragItem {
        equipped: false,
        handle: 7.try_into()?,
        count: 50,
    });
    let request = movement
        .drop_on(
            Some(ItemMoveDestination::Equipment(EquipmentSlot::Ammo)),
            true,
        )
        .ok_or("request")?;
    assert_eq!(request.count, None);
    assert!(movement.prompt.is_none());
    Ok(())
}

#[test]
fn equipped_ammo_unequips_whole_without_a_count_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let item = ItemEntry {
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 50.try_into()?,
        sub_type: 0,
    };
    let inventory = CharacterInventory {
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Ammo,
            item,
        }],
        ..Default::default()
    };
    let source = rows(Some(&inventory), true)[9]
        .drag_item()
        .ok_or("ammo source")?;
    let mut movement = crate::inventory::movement::Movement::default();
    assert!(movement.begin(source));
    let request = movement
        .drop_on(Some(ItemMoveDestination::MainBackpack), true)
        .ok_or("unequip request")?;
    assert_eq!(request.source, item.handle);
    assert_eq!(request.count, None);
    assert!(movement.prompt.is_none());
    Ok(())
}

#[test]
fn backpack_slot_is_only_a_drop_target() -> Result<(), Box<dyn std::error::Error>> {
    let inventory = CharacterInventory {
        main_backpack: Some(ItemEntry {
            handle: 7.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        }),
        ..Default::default()
    };
    let slot = rows(Some(&inventory), true)[2];
    assert!(slot.item.is_some());
    assert_eq!(slot.drag_item(), None);
    assert_eq!(slot.destination, ItemMoveDestination::MainBackpack);
    Ok(())
}

#[test]
fn equipment_is_unavailable_without_capability_even_with_an_inventory() {
    let inventory = CharacterInventory::default();
    for slot in rows(Some(&inventory), false) {
        assert!(!slot.available);
        assert!(slot.text().ends_with(": unavailable"));
        assert_eq!(slot.drag_item(), None);
    }
}
