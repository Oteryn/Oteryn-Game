use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
/// The highscores panel opens on the chord the player binds (Global has no default key).
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 11 && *modifiers == Modifiers::one(Modifier::Control)))
}
