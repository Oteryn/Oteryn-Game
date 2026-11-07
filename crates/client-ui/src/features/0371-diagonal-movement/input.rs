//! Native keypad diagonals; the caller gates these bindings on the negotiated profile.
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};
pub fn default_diagonal_binding() {}
pub fn keypad_step(events: &[NormalizedInputEvent], selected: bool) -> Option<StepDir> {
    if !selected {
        return None;
    }
    events.iter().find_map(|event| match event {
        NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            ..
        } if *modifiers == Modifiers::NONE => match code.get() {
            // Stable HID mappings already owned by input-platform::winit_adapter.
            95 => Some(StepDir::NorthWest),
            97 => Some(StepDir::NorthEast),
            89 => Some(StepDir::SouthWest),
            91 => Some(StepDir::SouthEast),
            _ => None,
        },
        _ => None,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
