//! Utility actions a player can assign to a custom hotkey. Each runs an existing client behaviour.
use super::{PreferenceError, ReferencePreferences, SettingsState};
use crate::play::PlayLink;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UtilityAction {
    ToggleCreatureHuds,
    ToggleFpsLagIndicator,
    NextHotkeyPreset,
    ClearOldestMessage,
    ShowIgnoreList,
    ToggleServerMessages,
    ChangeCharacter,
    BugReport,
    QuickLootArea,
    CloseCurrentChannel,
    NextChannel,
    OpenChannelList,
    OpenHelpChannel,
    PreviousChannel,
}

impl UtilityAction {
    pub const ALL: [Self; 14] = [
        Self::ToggleCreatureHuds,
        Self::ToggleFpsLagIndicator,
        Self::NextHotkeyPreset,
        Self::ClearOldestMessage,
        Self::ShowIgnoreList,
        Self::ToggleServerMessages,
        Self::ChangeCharacter,
        Self::BugReport,
        Self::QuickLootArea,
        Self::CloseCurrentChannel,
        Self::NextChannel,
        Self::OpenChannelList,
        Self::OpenHelpChannel,
        Self::PreviousChannel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ToggleCreatureHuds => "Utility: show or hide creature health bars",
            Self::ToggleFpsLagIndicator => "Utility: show or hide the FPS and lag indicator",
            Self::NextHotkeyPreset => "Utility: next hotkey preset",
            Self::ClearOldestMessage => "Utility: clear the oldest chat message",
            Self::ShowIgnoreList => "Utility: show the ignore, whitelist and blacklist",
            Self::ToggleServerMessages => "Utility: show or hide server messages in the console",
            Self::ChangeCharacter => "Utility: log out to change character",
            Self::BugReport => "Utility: open the bug report",
            Self::QuickLootArea => "Utility: quick loot around the player",
            Self::CloseCurrentChannel => "Utility: close the current chat channel",
            Self::NextChannel => "Utility: next chat channel",
            Self::OpenChannelList => "Utility: show the chat channel list",
            Self::OpenHelpChannel => "Utility: open the Help channel",
            Self::PreviousChannel => "Utility: previous chat channel",
        }
    }
}

impl ReferencePreferences {
    pub fn toggle_creature_huds(&mut self) {
        self.creature_bars = !self.creature_bars;
    }

    /// One switch for both readouts: any visible readout hides them, otherwise both show.
    pub fn toggle_fps_lag_indicator(&mut self) {
        let show = !(self.performance.show_fps || self.performance.show_latency);
        self.performance.show_fps = show;
        self.performance.show_latency = show;
    }

    /// Select the next preset by name, wrapping after the last.
    pub fn next_hotkey_preset(&mut self) -> Result<(), PreferenceError> {
        let names: Vec<String> = self.hotkey_contexts.presets.keys().cloned().collect();
        let position = names
            .iter()
            .position(|name| *name == self.hotkey_contexts.selected)
            .ok_or(PreferenceError)?;
        let next = names
            .get((position + 1) % names.len())
            .ok_or(PreferenceError)?
            .clone();
        self.select_hotkey_profile(&next)
    }
}

impl SettingsState {
    /// Preference actions edit and persist the settings; the rest use the live session.
    pub fn run_utility(&mut self, link: &PlayLink, action: UtilityAction) -> bool {
        let mut preferences = self.preferences.clone();
        match action {
            UtilityAction::ToggleCreatureHuds => preferences.toggle_creature_huds(),
            UtilityAction::ToggleFpsLagIndicator => preferences.toggle_fps_lag_indicator(),
            UtilityAction::NextHotkeyPreset => {
                if preferences.next_hotkey_preset().is_err() {
                    return false;
                }
            }
            _ => return link.utility_action(action),
        }
        self.apply(preferences);
        true
    }
}
