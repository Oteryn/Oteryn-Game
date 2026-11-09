//! Key editing for all 36 slots of the existing cg-options action-bar implementation.
use super::{CustomAction, PreferenceBinding, PreferenceError, ReferencePreferences};
use oteryn_client_runtime::spell::ACTION_BAR_SLOTS;

const EXTRA_IDS: &[&str] = &[
    "local-spell-action-bar-spell-5",
    "local-spell-action-bar-spell-6",
    "local-spell-action-bar-spell-7",
    "local-spell-action-bar-spell-8",
    "local-spell-action-bar-spell-9",
    "local-spell-action-bar-spell-10",
    "local-spell-action-bar-spell-11",
    "local-spell-action-bar-spell-12",
    "local-spell-action-bar-spell-13",
    "local-spell-action-bar-spell-14",
    "local-spell-action-bar-spell-15",
    "local-spell-action-bar-spell-16",
    "local-spell-action-bar-spell-17",
    "local-spell-action-bar-spell-18",
    "local-spell-action-bar-spell-19",
    "local-spell-action-bar-spell-20",
    "local-spell-action-bar-spell-21",
    "local-spell-action-bar-spell-22",
    "local-spell-action-bar-spell-23",
    "local-spell-action-bar-spell-24",
    "local-spell-action-bar-spell-25",
    "local-spell-action-bar-spell-26",
    "local-spell-action-bar-spell-27",
    "local-spell-action-bar-spell-28",
    "local-spell-action-bar-spell-29",
    "local-spell-action-bar-spell-30",
    "local-spell-action-bar-spell-31",
    "local-spell-action-bar-spell-32",
    "local-spell-action-bar-spell-33",
    "local-spell-action-bar-spell-34",
    "local-spell-action-bar-spell-35",
    "local-spell-action-bar-spell-36",
];
pub(super) fn extra_slot(id: &str) -> Option<u32> {
    id.strip_prefix("local-spell-action-bar-spell-")?
        .parse::<u32>()
        .ok()
        .filter(|slot| {
            usize::try_from(*slot).is_ok_and(|slot| (5..=ACTION_BAR_SLOTS).contains(&slot))
        })
}
pub(super) fn extra_bindings() -> Vec<PreferenceBinding> {
    EXTRA_IDS
        .iter()
        .filter(|id| extra_slot(id).is_some())
        .map(|id| PreferenceBinding {
            id,
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
                (slot - 1) / 12 + 1,
                (slot - 1) % 12 + 1
            )
        } else {
            self.id.to_owned()
        }
    }
}
impl ReferencePreferences {
    pub(super) fn bar_slot_chord(&self, slot: u32) -> &str {
        self.custom_hotkeys
            .iter()
            .find_map(|(chord, action)| {
                matches!(action, CustomAction::ActionBar { slot: bound } if *bound == slot)
                    .then_some(chord.as_str())
            })
            .unwrap_or("")
    }
    pub(super) fn rebind_bar_slot(
        &mut self,
        slot: u32,
        chord: &str,
    ) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        candidate.custom_hotkeys.retain(|_, action| !matches!(action, CustomAction::ActionBar { slot: bound } if *bound == slot));
        let chord = chord.trim().to_ascii_uppercase();
        if !chord.is_empty() {
            if candidate.custom_hotkeys.contains_key(&chord) {
                return Err(PreferenceError);
            }
            candidate.assign_custom_hotkey(&chord, CustomAction::ActionBar { slot })?;
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }
}
