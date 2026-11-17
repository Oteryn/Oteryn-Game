pub fn open_key(events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
    events.iter().any(|event| matches!(event,NormalizedInputEvent::Key {code,state:ButtonState::Pressed,modifiers,repeat:false} if code.get()==23 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}
