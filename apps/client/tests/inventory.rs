//! C08 synthetic native flow through the production mailbox and panel input.
use oteryn_client::play::play_channel;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{AppliedDelta, CharacterInventory, ItemEntry, SessionEvent};

fn item(handle: u64, count: u32) -> Result<ItemEntry, Box<dyn std::error::Error>> {
    Ok(ItemEntry {
        handle: handle.try_into()?,
        item_definition_ref: 2854.try_into()?,
        count: count.try_into()?,
        sub_type: 0,
    })
}

#[test]
fn live_inventory_selection_survives_updates_and_clears_removed_handles()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let backpack = item(1, 1)?;
    let stack = item(2, 7)?;
    let push = |value| {
        outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value,
        })])
    };
    push(CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![stack],
        equipment: vec![],
    });
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(12)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    assert!(link.inventory_input(&[key]));
    assert!(link.inventory_panel().open);
    assert_eq!(link.inventory_panel().rows(), vec![backpack, stack]);
    assert!(link.inventory_select(stack.handle));
    assert!(link.inventory_panel().text().contains("2854 ×7"));
    let changed = item(2, 3)?;
    push(CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![changed],
        equipment: vec![],
    });
    assert_eq!(link.inventory_panel().selected_item(), Some(changed));
    push(CharacterInventory::default());
    assert_eq!(link.inventory_panel().selected_item(), None);
    assert!(!link.inventory_select(stack.handle));
    drop(outbox);
    drop(link);
    Ok(())
}
