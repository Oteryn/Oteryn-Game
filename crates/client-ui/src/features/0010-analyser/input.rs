// oteryn-choice: CTRL+SHIFT+Y opens this local panel; text focus is handled by the shell.
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 28 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Super)))
}

/// Select a tool without resetting confirmed session measurements.
pub fn select_tool(panel: &mut super::Panel, key: &str) -> bool {
    if let Some(tool) = super::Tool::ALL
        .into_iter()
        .find(|tool| tool.key() == key.trim())
    {
        panel.tool = tool;
        true
    } else {
        false
    }
}
