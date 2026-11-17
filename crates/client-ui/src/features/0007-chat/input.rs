// CTRL+SHIFT+C preserves the public-channel shortcut.
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
#[must_use]
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 6 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift)
            && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Super)))
}
/// ALT+D (ShowDefaultChannel) returns the console to its default tab.
#[must_use]
pub fn default_channel_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 7 && *modifiers == Modifiers::one(Modifier::Alt)))
}
/// Keypad Enter (ChatModeTemporaryOn) switches the composer to temporary chat mode.
#[must_use]
pub fn temporary_mode_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 88 && *modifiers == Modifiers::NONE))
}
