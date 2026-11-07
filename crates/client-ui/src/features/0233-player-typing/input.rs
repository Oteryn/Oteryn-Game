use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

#[must_use]
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
            if code.get() == 23 && modifiers.contains(Modifier::Control)
                && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt)
                && !modifiers.contains(Modifier::Super))
    })
}

/// Coalesce edits and bound starts; stops never wait for an in-flight start.
/// Callers must keep calling `composer` or `tick` while a start is wanted;
/// the native composer polls focus/text every 30 ms, including unchanged text.
pub fn composer(
    panel: &mut super::PlayerTypingPanel,
    focused: bool,
    text: &str,
    now: std::time::Instant,
) -> Option<super::PlayerTypingCommand> {
    panel.start_wanted = panel.selected && focused && !text.is_empty();
    tick(panel, now)
}

/// Retry saved composer intent when the start rate gate opens.
pub fn tick(
    panel: &mut super::PlayerTypingPanel,
    now: std::time::Instant,
) -> Option<super::PlayerTypingCommand> {
    let typing = panel.start_wanted;
    if !panel.selected || typing == panel.composer_typing {
        return None;
    }
    if typing {
        if panel.last_start.is_some_and(|last| {
            now.saturating_duration_since(last) < std::time::Duration::from_millis(500)
        }) {
            return None;
        }
        panel.last_start = Some(now);
    }
    panel.composer_typing = typing;
    Some(super::PlayerTypingCommand {
        typing,
        origin: super::PlayerTypingOrigin::Composer,
    })
}
