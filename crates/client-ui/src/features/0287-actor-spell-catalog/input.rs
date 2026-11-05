// oteryn-choice: CTRL+SHIFT+U opens this local panel; text focus is handled by the shell.
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 24 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Super)))
}
