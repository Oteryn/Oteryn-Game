use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};

/// Unmodified digits 1 through 5; key repeat never submits another choice.
pub fn choice_key(events: &[NormalizedInputEvent]) -> Option<u8> {
    events.iter().find_map(|event| match event {
        NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        } if *modifiers == Modifiers::NONE && (30..=34).contains(&code.get()) => {
            u8::try_from(code.get() - 29).ok()
        }
        _ => None,
    })
}
