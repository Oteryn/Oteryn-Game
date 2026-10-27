// Logout key intent mapping.
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
const KEY_L: u16 = 15;
const KEY_Q: u16 = 20;
/// Registration marker for logout defaults.
pub fn default_logout_binding() {}
#[must_use]
pub fn logout_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event|matches!(event,NormalizedInputEvent::Key{code,state:ButtonState::Pressed,modifiers,repeat:false} if matches!(code.get(),KEY_L|KEY_Q)&&modifiers.contains(Modifier::Control)))
}
