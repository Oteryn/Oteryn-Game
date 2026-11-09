use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};
/// Map negotiated diagonal keypad input to the existing paced movement intent.
pub fn step(events: &[NormalizedInputEvent], selected: bool) -> Option<StepDir> {
    if !selected {
        return None;
    }
    events.iter().find_map(|event| {
        let NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            ..
        } = event
        else {
            return None;
        };
        if *modifiers != Modifiers::NONE {
            return None;
        }
        match code.get() {
            95 => Some(StepDir::NorthWest),
            97 => Some(StepDir::NorthEast),
            89 => Some(StepDir::SouthWest),
            91 => Some(StepDir::SouthEast),
            _ => None,
        }
    })
}
