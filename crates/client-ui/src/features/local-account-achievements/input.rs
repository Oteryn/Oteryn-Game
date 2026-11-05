use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
/// Ctrl+Shift+A opens account achievements; repeat never sends a query.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 4 && *modifiers == Modifiers::one(Modifier::Control).with(Modifier::Shift))
    })
}
