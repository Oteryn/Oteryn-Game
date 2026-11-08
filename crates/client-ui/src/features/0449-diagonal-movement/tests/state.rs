use super::*;
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
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
        assert_eq!(step(&[event.clone()], false), None);
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
