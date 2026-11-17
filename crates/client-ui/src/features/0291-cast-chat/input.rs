// oteryn-choice: an unused native chord opens the isolated room.
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    let chord = Modifiers::one(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    events.iter().any(|event|matches!(event,NormalizedInputEvent::Key{code,state:ButtonState::Pressed,modifiers,repeat:false}if code.get()==11&&*modifiers==chord))
}
