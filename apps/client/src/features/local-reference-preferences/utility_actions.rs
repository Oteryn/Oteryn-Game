//! Utility actions a player can assign to a custom hotkey. Each runs an existing client behaviour.
use super::{PreferenceError, ReferencePreferences, SettingsPage, SettingsState};
use crate::play::PlayLink;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UtilityAction {
    ToggleCreatureHuds,
    ToggleFpsLagIndicator,
    ToggleDiagnosticsOverlay,
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
    ToggleLensHelp,
    ShowPrey,
    ToggleVipWidget,
    ToggleSkillsWidget,
    ShowOptionsHotkeys,
    ToggleManualSortMode,
}

impl UtilityAction {
    pub const ALL: [Self; 21] = [
        Self::ToggleCreatureHuds,
        Self::ToggleFpsLagIndicator,
        Self::ToggleDiagnosticsOverlay,
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
        Self::ToggleLensHelp,
        Self::ShowPrey,
        Self::ToggleVipWidget,
        Self::ToggleSkillsWidget,
        Self::ShowOptionsHotkeys,
        Self::ToggleManualSortMode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ToggleCreatureHuds => "Utility: show or hide creature health bars",
            Self::ToggleFpsLagIndicator => "Utility: show or hide the FPS and lag indicator",
            Self::ToggleDiagnosticsOverlay => "Utility: show or hide the diagnostics overlay",
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
            Self::ToggleLensHelp => "Utility: show or hide the lens help",
            Self::ShowPrey => "Utility: show the Prey dialog",
            Self::ToggleVipWidget => "Utility: show or hide the VIP list",
            Self::ToggleSkillsWidget => "Utility: show or hide the skills window",
            Self::ShowOptionsHotkeys => "Utility: show the hotkey options",
            Self::ToggleManualSortMode => "Utility: use or stop manual container sort mode",
        }
    }
}

impl ReferencePreferences {
    pub fn toggle_creature_huds(&mut self) {
        self.creature_bars = !self.creature_bars;
    }

    /// One switch for both readouts: any visible readout hides them, otherwise both show.
    pub fn toggle_fps_lag_indicator(&mut self) {
        let mut performance = self.render_performance();
        let show = !(performance.show_fps || performance.show_latency);
        performance.show_fps = show;
        performance.show_latency = show;
        self.foreground_fps = performance.foreground_fps;
        self.background_fps = performance.background_fps;
        self.frames = performance.into();
        self.performance = performance;
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
            UtilityAction::ToggleDiagnosticsOverlay => {
                preferences.diagnostics.overlay = !preferences.diagnostics.overlay;
            }
            UtilityAction::NextHotkeyPreset => {
                if preferences.next_hotkey_preset().is_err() {
                    return false;
                }
            }
            UtilityAction::ShowOptionsHotkeys => {
                self.request_page(SettingsPage::Hotkeys);
                return true;
            }
            _ => return link.utility_action(action),
        }
        self.apply(preferences);
        true
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    #[test]
    fn indicator_toggle_and_basic_editor_update_rendered_performance()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _, _) = crate::play::play_channel();
        let mut state = SettingsState::new(None);
        state.preferences.frames.foreground = 144;
        assert!(state.run_utility(&link, UtilityAction::ToggleFpsLagIndicator));
        let rendered = state.preferences.render_performance();
        assert!(rendered.show_fps && rendered.show_latency);
        assert_eq!(rendered.foreground_fps, 144);
        assert!(state.run_utility(&link, UtilityAction::ToggleFpsLagIndicator));
        assert!(!state.preferences.render_performance().show_fps);
        state.preferences = ReferencePreferences::default();
        state.preferences.frames.show_fps = true;
        state.preferences.foreground_fps = 144;
        assert!(state.run_utility(&link, UtilityAction::ToggleFpsLagIndicator));
        assert_eq!(state.preferences.render_performance().foreground_fps, 60);
        state.preferences = ReferencePreferences::default();
        state.preferences.performance.foreground_fps = 240;
        state.preferences.performance.show_latency = true;
        let basic = super::super::BasicOptions::from(&state.preferences);
        state.apply_basic(&basic)?;
        assert_eq!(
            state.preferences.render_performance().foreground_fps,
            basic.foreground_fps
        );
        assert!(!state.preferences.render_performance().show_latency);
        Ok(())
    }
}
