use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

/// Ctrl+A opens Animus Mastery. Repeated presses and other chords do not query.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
            if code.get() == 4 && modifiers.contains(Modifier::Control)
                && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift)
                && !modifiers.contains(Modifier::Super))
    })
}
