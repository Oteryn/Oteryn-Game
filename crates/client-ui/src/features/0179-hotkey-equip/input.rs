use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 8 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift)))
}

use super::HotkeyEquipPanel;
use std::num::NonZeroU32;

pub const ITEM_HOTKEY_SLOTS: usize = 12;

pub fn assign(panel: &mut HotkeyEquipPanel, slot: usize, definition: u32) -> bool {
    if slot >= ITEM_HOTKEY_SLOTS || (definition != 0 && !panel.items.contains(&definition)) {
        return false;
    }
    panel.bindings[slot] = NonZeroU32::new(definition);
    true
}
