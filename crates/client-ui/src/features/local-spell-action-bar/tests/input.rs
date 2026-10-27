use super::SpellHotkeys;
use oteryn_input_actions::{ButtonState, InputError, KeyCode, Modifiers, NormalizedInputEvent};

fn key(code: u16, state: ButtonState) -> Result<NormalizedInputEvent, InputError> {
    Ok(NormalizedInputEvent::Key {
        code: KeyCode::new(code)?,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    })
}

#[test]
fn digit_hotkeys_route_to_spell_book_indices() -> Result<(), InputError> {
    let mut hotkeys = SpellHotkeys::new()?;
    let pressed = hotkeys.route(&[key(31, ButtonState::Pressed)?]);
    assert_eq!(pressed.iter().map(|s| s.get()).collect::<Vec<_>>(), [2]);
    hotkeys.route(&[key(31, ButtonState::Released)?]);
    // Outside the bar (digit 5) and while text is active nothing routes.
    assert!(hotkeys.route(&[key(34, ButtonState::Pressed)?]).is_empty());
    hotkeys.route(&[key(34, ButtonState::Released)?]);
    hotkeys.set_text_active(true)?;
    assert!(hotkeys.route(&[key(30, ButtonState::Pressed)?]).is_empty());
    Ok(())
}
