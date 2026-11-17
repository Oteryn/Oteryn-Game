use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
/// Ctrl+Alt+Shift+U opens character effects; repeats and extra modifiers do not reopen it.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 24 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}
