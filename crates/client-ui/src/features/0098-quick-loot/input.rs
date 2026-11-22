use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key {code,state: ButtonState::Pressed,modifiers,repeat: false} if code.get() == 20 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

/// Loot nearby containers without opening the configuration panel.
pub fn nearby_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event, NormalizedInputEvent::Key {
        code, state: ButtonState::Pressed, modifiers, repeat: false
    } if code.get() == 20 && *modifiers == Modifiers::one(Modifier::Alt))
    })
}
