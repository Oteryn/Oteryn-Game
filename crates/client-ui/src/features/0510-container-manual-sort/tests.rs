use super::*;

#[test]
fn a_slot_is_sent_only_while_the_negotiated_mode_is_on() -> Result<(), Box<dyn std::error::Error>> {
    let handle = 9.try_into()?;
    let slotted = ItemMoveDestination::Container(handle, Some(2));
    let unslotted = ItemMoveDestination::Container(handle, None);

    let mut unnegotiated = admitted(false);
    assert!(!unnegotiated.toggle());
    assert!(!unnegotiated.active());
    assert_eq!(drop_destination(unnegotiated, slotted), unslotted);

    let mut mode = admitted(true);
    assert!(!mode.active());
    assert_eq!(drop_destination(mode, slotted), unslotted);
    assert!(mode.toggle());
    assert!(mode.active());
    assert_eq!(drop_destination(mode, slotted), slotted);
    assert_eq!(
        drop_destination(mode, ItemMoveDestination::MainBackpack),
        ItemMoveDestination::MainBackpack
    );
    assert!(mode.toggle());
    assert_eq!(drop_destination(mode, slotted), unslotted);
    Ok(())
}

/// A list drop surface (the Windows-native container and inventory windows) names the row it was
/// dropped on while the mode is on: a container's slot, or the main backpack's slot by its handle.
#[test]
fn a_dropped_on_list_row_names_its_slot_only_in_the_mode() -> Result<(), Box<dyn std::error::Error>>
{
    let container = 9.try_into()?;
    let backpack = 4.try_into()?;
    let in_view = ItemMoveDestination::Container(container, None);
    let mut mode = admitted(true);
    assert_eq!(
        row_destination(mode, in_view, Some(backpack), Some(3)),
        in_view
    );
    assert!(mode.toggle());
    assert_eq!(
        row_destination(mode, in_view, Some(backpack), Some(3)),
        ItemMoveDestination::Container(container, Some(3))
    );
    assert_eq!(
        row_destination(
            mode,
            ItemMoveDestination::MainBackpack,
            Some(backpack),
            Some(1)
        ),
        ItemMoveDestination::Container(backpack, Some(1))
    );
    // No row under the pointer, or no known main backpack: the default placement.
    assert_eq!(
        row_destination(mode, in_view, Some(backpack), None),
        in_view
    );
    assert_eq!(
        row_destination(mode, ItemMoveDestination::MainBackpack, None, Some(1)),
        ItemMoveDestination::MainBackpack
    );
    let equipment =
        ItemMoveDestination::Equipment(oteryn_client_runtime::equipment::EquipmentSlot::Armor);
    assert_eq!(
        row_destination(mode, equipment, Some(backpack), Some(0)),
        equipment
    );
    Ok(())
}
