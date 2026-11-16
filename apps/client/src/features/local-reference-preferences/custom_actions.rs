//! Durable custom actions contain definitions, never connection-local item handles.
use super::{PreferenceError, ReferencePreferences};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectTarget {
    Use,
    Cursor,
    SelfActor,
    CurrentTarget,
    Equip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustomAction {
    PassiveStatus {
        ability: crate::play::PassiveAbility,
    },
    ActionBar {
        slot: u32,
    },
    Text {
        text: String,
        send: bool,
    },
    Object {
        definition: u32,
        sub_type: u32,
        target: ObjectTarget,
    },
    /// Read-only tracking, including abilities not yet learned. Never casts a spell.
    Passive {
        cast_index: u32,
    },
    Utility {
        action: super::UtilityAction,
    },
}
impl CustomAction {
    pub fn hotkey_label(&self, key: &str) -> String {
        if matches!(self, Self::Object { target: ObjectTarget::Cursor, .. }) {
            format!("{key}: (use object with crosshair)")
        } else {
            key.to_owned()
        }
    }

    pub(super) fn valid(&self) -> bool {
        match self {
            Self::PassiveStatus { .. } | Self::Utility { .. } => true,
            Self::ActionBar { slot } => usize::try_from(*slot).is_ok_and(|slot| {
                (1..=oteryn_client_runtime::spell::ACTION_BAR_SLOTS).contains(&slot)
            }),
            Self::Text { text, .. } => {
                !text.trim().is_empty() && text.len() <= 1020 && !text.contains('\0')
            }
            Self::Object { definition, .. } => *definition > 0,
            Self::Passive { cast_index } => (1..1 << 28).contains(cast_index),
        }
    }
}
impl ReferencePreferences {
    pub fn assign_custom_hotkey(
        &mut self,
        chord: &str,
        action: CustomAction,
    ) -> Result<(), PreferenceError> {
        if self
            .custom_hotkeys
            .contains_key(&chord.trim().to_ascii_uppercase())
        {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        candidate
            .custom_hotkeys
            .insert(chord.trim().to_ascii_uppercase(), action);
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }
}

impl ReferencePreferences {
    pub(super) fn validate_custom_chords(
        &self,
        primary: &std::collections::BTreeMap<String, String>,
        secondary: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), PreferenceError> {
        super::bindings::validate_custom(self, primary, secondary)
    }
}

/// The chat, help, window and container chords of Tibia Global, bound on a fresh profile. The
/// features that held Ctrl+E, Ctrl+O, Ctrl+T, Ctrl+H, Ctrl+M, Ctrl+P, Ctrl+V, Ctrl+K and Ctrl+Y
/// before have no default key now; Global gives them none.
pub fn default_custom_hotkeys() -> std::collections::BTreeMap<String, CustomAction> {
    use super::UtilityAction::{
        CloseCurrentChannel, NextChannel, OpenChannelList, OpenHelpChannel, PreviousChannel,
        ShowOptionsHotkeys, ShowPrey, ToggleLensHelp, ToggleManualSortMode, ToggleServerMessages,
        ToggleSkillsWidget, ToggleVipWidget,
    };
    [
        ("CTRL+E", CloseCurrentChannel),
        ("CTRL+M", ToggleServerMessages),
        ("TAB", NextChannel),
        ("SHIFT+TAB", PreviousChannel),
        ("CTRL+O", OpenChannelList),
        ("CTRL+T", OpenHelpChannel),
        ("CTRL+H", ToggleLensHelp),
        ("CTRL+Y", ShowPrey),
        ("CTRL+P", ToggleVipWidget),
        ("ALT+S", ToggleSkillsWidget),
        ("CTRL+K", ShowOptionsHotkeys),
        ("SHIFT+S", ToggleManualSortMode),
    ]
    .into_iter()
    .map(|(chord, action)| (chord.to_owned(), CustomAction::Utility { action }))
    .collect()
}
