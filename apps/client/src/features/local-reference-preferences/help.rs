//! Local help navigation. Opening a report never submits one on the player's behalf.
use super::SettingsState;
use crate::play::PlayLink;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpTopic {
    Questions,
    Manual,
    Client,
}

pub fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::Questions => {
            "Frequently asked questions\r\n\r\nWhy is a feature unavailable? Panels depend on the capabilities admitted by the current server. An unavailable panel does not grant access to that feature.\r\n\r\nWhere are my settings? Each admitted character has separate local settings. Before admission, changes configure defaults for new characters.\r\n\r\nHow do I recover settings? Settings offers export, import, defaults and a backup of the settings replaced by import or reset. Export creates a new file; choose a new filename if one already exists.\r\n\r\nWhy did an action fail? Check the panel's server reply. Inventory, combat, cooldowns and movement remain controlled by the server."
        }
        HelpTopic::Manual => {
            "Playing Oteryn\r\n\r\nUse the arrow keys to move. Click a visible world tile to select it; the minimap captures clicks within its rectangle for walking. Server replies confirm movement.\r\n\r\nOpen Settings from the window menu. Its action list shows the current keyboard shortcuts. Hotkey profiles lets you copy, rename or remove a preset and edit primary and secondary keys separately while chat is open or closed. Conflicting keys are rejected.\r\n\r\nBattle lists can be named, filtered and sorted independently. Skills shows server-provided trained values, loyalty and progress; its controls change the displayed order and number style.\r\n\r\nUse the spell action bar to assign known spells. Casting, costs and cooldowns require server acceptance. Consult each gameplay panel for its current state and action feedback."
        }
        HelpTopic::Client => {
            "Client help and troubleshooting\r\n\r\nSettings contains world zoom, master volume, mute, hotkey profiles and settings transfer. Changes apply immediately; a save error is reported and changes stay active for this session.\r\n\r\nIf keys stop responding, focus the game window again. Focus loss clears held-key state. Verify the active hotkey profile and its chat context before rebinding.\r\n\r\nFor a display problem, include the window size and the panel involved in your report. For a gameplay problem, include what you did, the expected result and the actual server feedback. Never include passwords or sign-in tokens.\r\n\r\nOpen Reports from this Help page to choose the existing bug or player-violation form. Review the details and submit explicitly; opening the form sends no report."
        }
    }
}

impl SettingsState {
    pub fn request_help_report(&mut self) {
        self.help_report_pending = true;
    }

    /// Called on the game thread, where the report window and live session belong.
    pub fn open_help_report(&mut self, link: Option<&PlayLink>) {
        if !std::mem::take(&mut self.help_report_pending) {
            return;
        }
        if let Some(link) = link {
            link.player_report_open();
            self.feedback =
                "Reports opened. Choose a report type and review it before submitting.".into();
        } else {
            self.feedback = "Join a game session before opening its report form.".into();
        }
    }
}
