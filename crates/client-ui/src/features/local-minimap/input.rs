use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn editor_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 16 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}
/// `Misc.: Create Mark`: Alt+Shift+M creates or edits the mark at the player's position.
pub fn create_mark_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 16 && modifiers.contains(Modifier::Alt) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Super)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::{KeyCode, Modifiers};
    #[test]
    fn editor_shortcut_requires_the_exact_chord_and_a_fresh_press()
    -> Result<(), oteryn_input_actions::InputError> {
        let modifiers = Modifiers::NONE
            .with(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        let key = |modifiers, repeat| {
            Ok::<_, oteryn_input_actions::InputError>(NormalizedInputEvent::Key {
                code: KeyCode::new(16)?,
                state: ButtonState::Pressed,
                modifiers,
                repeat,
            })
        };
        assert!(editor_key(&[key(modifiers, false)?]));
        assert!(!editor_key(&[key(modifiers, true)?]));
        assert!(!editor_key(&[key(modifiers.with(Modifier::Super), false)?]));
        assert!(!editor_key(&[key(Modifiers::NONE, false)?]));
        Ok(())
    }
    #[test]
    fn create_mark_shortcut_requires_alt_shift_m_and_a_fresh_press()
    -> Result<(), oteryn_input_actions::InputError> {
        let modifiers = Modifiers::NONE.with(Modifier::Alt).with(Modifier::Shift);
        let key = |modifiers, repeat| {
            Ok::<_, oteryn_input_actions::InputError>(NormalizedInputEvent::Key {
                code: KeyCode::new(16)?,
                state: ButtonState::Pressed,
                modifiers,
                repeat,
            })
        };
        assert!(create_mark_key(&[key(modifiers, false)?]));
        assert!(!create_mark_key(&[key(modifiers, true)?]));
        assert!(!create_mark_key(&[key(
            modifiers.with(Modifier::Control),
            false
        )?]));
        assert!(!create_mark_key(&[key(Modifiers::NONE, false)?]));
        assert!(!editor_key(&[key(modifiers, false)?]));
        Ok(())
    }
}
