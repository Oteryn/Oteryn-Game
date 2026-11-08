// Keyboard-to-movement intent mapping.
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, NormalizedInputEvent};
/// Registration marker for each arrow-key default.
pub fn default_arrow_binding() {}
#[must_use]
pub fn arrow_step(events: &[NormalizedInputEvent]) -> Option<StepDir> {
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
                95 => Some(StepDir::NorthWest),
                97 => Some(StepDir::NorthEast),
                89 => Some(StepDir::SouthWest),
                91 => Some(StepDir::SouthEast),
                _ => None,
            },
        },
        _ => None,
    })
}

#[cfg(test)]
#[test]
fn qn6_keypad_diagonal_press_maps_to_one_step() -> Result<(), oteryn_input_actions::InputError> {
    for (key,direction) in [(95,StepDir::NorthWest),(97,StepDir::NorthEast),
        (89,StepDir::SouthWest),(91,StepDir::SouthEast)] {
        let event=NormalizedInputEvent::Key { code:KeyCode::new(key)?,state:ButtonState::Pressed,
            modifiers:oteryn_input_actions::Modifiers::default(),repeat:false };
        assert_eq!(arrow_step(&[event]),Some(direction));
    }
    Ok(())
}
