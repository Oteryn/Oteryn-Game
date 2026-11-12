use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
/// Ctrl+N opens the panel; repeats and releases do not issue queries.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 17 && *modifiers == Modifiers::one(Modifier::Control))
    })
}
