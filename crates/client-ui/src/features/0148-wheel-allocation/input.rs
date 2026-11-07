use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

/// Ctrl+W opens the Wheel panel, ignoring repeats and releases.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 26 && modifiers.contains(Modifier::Control)
            && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift)
            && !modifiers.contains(Modifier::Super))
    })
}
