use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
/// Ctrl+Shift+Z opens the arena; Ctrl+Shift+A remains account achievements.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 29 && modifiers.contains(Modifier::Control)
        && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt)
        && !modifiers.contains(Modifier::Super))
    })
}
