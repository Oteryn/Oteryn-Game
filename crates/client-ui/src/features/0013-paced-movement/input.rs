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
            _ => None,
        },
        _ => None,
    })
}
