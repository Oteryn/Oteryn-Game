use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key {code, state: ButtonState::Pressed, modifiers, repeat: false}
        if code.get() == 8 && *modifiers == chord)
    })
}
