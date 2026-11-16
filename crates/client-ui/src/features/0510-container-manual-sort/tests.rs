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
