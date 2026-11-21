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

#[test]
fn all_default_spell_hotkeys_repeat_until_release_or_text_focus() -> Result<(), InputError> {
    for slot in 1..=super::SPELL_SLOTS {
        let mut hotkeys = SpellHotkeys::new()?;
        let code = super::FIRST_DIGIT_KEY + slot - 1;
        let press = key(code, ButtonState::Pressed)?;
        let repeat = NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: true,
        };
        assert_eq!(
            hotkeys.route(std::slice::from_ref(&press))[0].get(),
            u32::from(slot)
        );
        for _ in 0..3 {
            let routed = hotkeys.route(std::slice::from_ref(&repeat));
            assert_eq!(routed.len(), 1, "held slot {slot}");
            assert_eq!(routed[0].get(), u32::from(slot));
        }
        assert!(
            hotkeys
                .route(&[key(code, ButtonState::Released)?])
                .is_empty()
        );
        assert!(hotkeys.route(std::slice::from_ref(&repeat)).is_empty());
        hotkeys.route(std::slice::from_ref(&press));
        hotkeys.set_text_active(true)?;
        assert!(hotkeys.route(std::slice::from_ref(&repeat)).is_empty());
    }
    Ok(())
}
