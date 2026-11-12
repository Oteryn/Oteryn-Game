use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key {code, state: ButtonState::Pressed, modifiers, repeat: false}
        if code.get() == 13 && *modifiers == chord)
    })
}

pub fn titles_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 28 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}
