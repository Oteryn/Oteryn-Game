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

#[test]
fn m40_keypad_diagonals_require_selected_capability() -> Result<(), Box<dyn std::error::Error>> {
    for (code, direction, expected_facing) in [
        (95, StepDir::NorthWest, 3),
        (97, StepDir::NorthEast, 1),
        (89, StepDir::SouthWest, 3),
        (91, StepDir::SouthEast, 1),
    ] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(step(std::slice::from_ref(&event), false), None);
        assert_eq!(step(&[event], true), Some(direction));
        assert_eq!(facing(direction), expected_facing);
        let released = NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(step(&[released], true), None);
    }
    Ok(())
}
