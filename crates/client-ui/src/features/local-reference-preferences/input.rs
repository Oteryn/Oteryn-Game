//! The settings action is local and remains reachable without a negotiated capability.
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};

pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event, NormalizedInputEvent::Key {
        code, state: ButtonState::Pressed, modifiers, repeat: false
    } if code.get() == 18 && *modifiers == Modifiers::one(Modifier::Control).with(Modifier::Shift))
    })
}
