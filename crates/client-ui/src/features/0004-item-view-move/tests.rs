use super::*;
use oteryn_input_actions::{KeyCode, Modifiers};

#[test]
fn inventory_selection_tracks_handles_instead_of_rows() -> Result<(), Box<dyn std::error::Error>> {
    let entry = ItemEntry {
        handle: 7.try_into()?,
        item_definition_ref: 100.try_into()?,
        count: 2.try_into()?,
        sub_type: 5,
    };
    let mut panel = InventoryPanel::default();
    assert!(panel.text().contains("unavailable"));
    assert!(!panel.select(entry.handle));
    panel.apply(Some(CharacterInventory {
        main_backpack: Some(entry),
        ..Default::default()
    }));
    assert!(panel.select(entry.handle));
    let changed = ItemEntry {
        count: 9.try_into()?,
        ..entry
    };
    panel.apply(Some(CharacterInventory {
        main_backpack: Some(changed),
        ..Default::default()
    }));
    assert_eq!(panel.selected_item(), Some(changed));
    assert!(panel.text().contains("100 ×9"));
    panel.apply(Some(CharacterInventory::default()));
    assert_eq!(panel.selected_item(), None);
    assert_eq!(panel.text(), "Backpack empty");
    panel.apply(None);
    assert_eq!(panel.rows(), vec![]);
    Ok(())
}

#[test]
fn inventory_open_key_is_exact_and_ignores_repeats() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(12)?;
    let key = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(open_key(&[key(ButtonState::Pressed, ctrl, false)]));
    assert!(!open_key(&[key(ButtonState::Released, ctrl, false)]));
    assert!(!open_key(&[key(ButtonState::Pressed, ctrl, true)]));
    assert!(!open_key(&[key(
        ButtonState::Pressed,
        Modifiers::default(),
        false
    )]));
    for modifier in [Modifier::Alt, Modifier::Shift, Modifier::Super] {
        assert!(!open_key(&[key(
            ButtonState::Pressed,
            ctrl.with(modifier),
            false
        )]));
    }
    Ok(())
}
