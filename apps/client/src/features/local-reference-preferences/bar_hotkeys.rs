//! Key editing for all action-bar slots of the existing cg-options action-bar implementation.
use super::{CustomAction, PreferenceBinding, PreferenceError, ReferencePreferences};
use oteryn_client_runtime::spell::ACTION_BAR_SLOTS;

pub(super) fn extra_slot(id: &str) -> Option<u32> {
    id.strip_prefix("local-spell-action-bar-spell-")?
        .parse::<u32>()
        .ok()
        .filter(|slot| {
            usize::try_from(*slot).is_ok_and(|slot| (5..=ACTION_BAR_SLOTS).contains(&slot))
        })
}
pub(super) fn extra_bindings() -> Vec<PreferenceBinding> {
    static IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        (5..=ACTION_BAR_SLOTS)
            .map(|slot| format!("local-spell-action-bar-spell-{slot}"))
            .collect()
    })
    .iter()
    .filter(|id| extra_slot(id).is_some())
    .map(|id| PreferenceBinding {
        id: id.as_str(),
        default_chord: "",
    })
    .collect()
}
impl PreferenceBinding {
    pub fn label(&self) -> String {
        if let Some(slot) = self
            .id
            .strip_prefix("local-spell-action-bar-spell-")
            .and_then(|slot| slot.parse::<usize>().ok())
            .filter(|slot| (1..=ACTION_BAR_SLOTS).contains(slot))
        {
            format!(
                "Action bar row {} / slot {}",
                (slot - 1) / 50 + 1,
                (slot - 1) % 50 + 1
            )
        } else {
            self.id.to_owned()
        }
    }
}
impl ReferencePreferences {
    pub(super) fn bar_slot_chord(&self, slot: u32) -> &str {
        let key = usize::try_from(slot)
            .ok()
            .and_then(|value| value.checked_sub(1));
        key.and_then(|key| self.action_keys.0.get(&key))
            .map(String::as_str)
            .unwrap_or_else(|| {
                self.custom_hotkeys
                    .iter()
                    .find_map(|(chord, action)| {
                        matches!(action, CustomAction::ActionBar { slot: bound } if *bound == slot)
                            .then_some(chord.as_str())
                    })
                    .unwrap_or("")
            })
    }
    pub(super) fn rebind_bar_slot(
        &mut self,
        slot: u32,
        chord: &str,
    ) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        candidate.custom_hotkeys.retain(|_, action| !matches!(action, CustomAction::ActionBar { slot: bound } if *bound == slot));
        let index = usize::try_from(slot)
            .ok()
            .and_then(|value| value.checked_sub(1))
            .ok_or(PreferenceError)?;
        let chord = chord.trim().to_ascii_uppercase();
        if !chord.is_empty() {
            candidate.action_keys.0.insert(index, chord);
        } else {
            candidate.action_keys.0.remove(&index);
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }
}

pub(super) fn routed_action(
    preferences: &ReferencePreferences,
    code: oteryn_input_actions::KeyCode,
    modifiers: oteryn_input_actions::Modifiers,
    chat_on: bool,
) -> Option<CustomAction> {
    use oteryn_input_actions::Modifier;
    if chat_on
        && ((4..=39).contains(&code.get()) || code.get() == 44)
        && ![Modifier::Control, Modifier::Alt, Modifier::Super]
            .iter()
            .any(|modifier| modifiers.contains(*modifier))
    {
        return None;
    }
    let matches = |key: &str| super::bindings::parse_action_chord(key) == Some((code, modifiers));
    preferences
        .custom_hotkeys
        .iter()
        .find(|(key, _)| matches(key))
        .map(|(_, action)| action.clone())
        .or_else(|| {
            preferences
                .action_keys
                .0
                .iter()
                .find(|(_, key)| matches(key))
                .and_then(|(slot, _)| u32::try_from(slot + 1).ok())
                .map(|slot| CustomAction::ActionBar { slot })
        })
}
