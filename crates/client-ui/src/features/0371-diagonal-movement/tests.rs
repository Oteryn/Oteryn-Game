use super::*;
use oteryn_input_actions::KeyCode;
#[test]
fn negotiated_profile_exposes_only_native_keypad_diagonals() -> Result<(), String> {
    for (code, direction) in [
        (95, StepDir::NorthWest),
        (97, StepDir::NorthEast),
        (89, StepDir::SouthWest),
        (91, StepDir::SouthEast),
    ] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(code).map_err(|error| format!("{error:?}"))?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(keypad_step(std::slice::from_ref(&event), false), None);
        assert_eq!(
            keypad_step(std::slice::from_ref(&event), true),
            Some(direction)
        );
    }
    assert_eq!(keypad_step(&[], true), None);
    let release = NormalizedInputEvent::Key {
        code: KeyCode::new(95).map_err(|error| format!("{error:?}"))?,
        state: ButtonState::Released,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert_eq!(keypad_step(&[release], true), None);
    Ok(())
}
