// Keyboard-to-movement intent mapping.
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, NormalizedInputEvent};
/// Registration marker for each arrow-key default.
pub fn default_arrow_binding() {}
#[must_use]
pub fn arrow_step(events: &[NormalizedInputEvent], diagonal_selected: bool) -> Option<StepDir> {
    events.iter().find_map(|event| match event {
        NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            ..
        } => match *code {
            KeyCode::ARROW_UP => Some(StepDir::North),
            KeyCode::ARROW_RIGHT => Some(StepDir::East),
            KeyCode::ARROW_DOWN => Some(StepDir::South),
            KeyCode::ARROW_LEFT => Some(StepDir::West),
            code => match code.get() {
                96 => Some(StepDir::North),
                94 => Some(StepDir::East),
                90 => Some(StepDir::South),
                92 => Some(StepDir::West),
                95 if diagonal_selected => Some(StepDir::NorthWest),
                97 if diagonal_selected => Some(StepDir::NorthEast),
                89 if diagonal_selected => Some(StepDir::SouthWest),
                91 if diagonal_selected => Some(StepDir::SouthEast),
                _ => None,
            },
        },
        _ => None,
    })
}

#[cfg(test)]
#[test]
fn qn6_keypad_diagonal_press_maps_to_one_step() -> Result<(), oteryn_input_actions::InputError> {
    for (key, direction) in [
        (95, StepDir::NorthWest),
        (97, StepDir::NorthEast),
        (89, StepDir::SouthWest),
        (91, StepDir::SouthEast),
    ] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(key)?,
            state: ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::default(),
            repeat: false,
        };
        assert_eq!(arrow_step(&[event], true), Some(direction));
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn review12_keypad_diagonals_require_selected_capability() -> Result<(), oteryn_input_actions::InputError> {
    for key in [95, 97, 89, 91] {
        let event = NormalizedInputEvent::Key { code: KeyCode::new(key)?, state: ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::default(), repeat: false };
        assert_eq!(arrow_step(&[event], false), None);
    }
    Ok(())
}
