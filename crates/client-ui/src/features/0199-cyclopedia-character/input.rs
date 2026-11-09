use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
#[must_use]
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 24 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

/// Empty selects self; otherwise use the native contract's 16-byte Character id.
pub fn set_target(p: &mut super::CyclopediaCharacterPanel, text: &str) -> bool {
    if p.pending {
        p.feedback = Some(tr!("Waiting for the server"));
        return false;
    }
    let text = text.trim();
    let target = if text.is_empty() {
        None
    } else {
        if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            p.feedback = Some(tr!(
                "Enter 32 hexadecimal Character id digits, or leave empty for self"
            ));
            return false;
        }
        let mut id = [0; 16];
        for (i, byte) in id.iter_mut().enumerate() {
            let Ok(value) = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16) else {
                return false;
            };
            *byte = value;
        }
        Some(id)
    };
    p.change_target(target);
    p.offset = 0;
    p.result = None;
    p.feedback = None;
    true
}
