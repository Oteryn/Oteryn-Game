use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

/// Ctrl+Shift+L opens the depot panel; repeats and modified chords are ignored.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
            NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
            if code.get() == 15 && modifiers.contains(Modifier::Control)
                && !modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift)
                && !modifiers.contains(Modifier::Super))
    })
}
