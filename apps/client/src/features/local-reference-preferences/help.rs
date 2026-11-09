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
// Settings navigation reuses the Oteryn player guide and the reviewed report callers.
use super::{WINDOW_ACTIONS, preference_bindings};
use oteryn_client_ui::guide::{Guide, Page};

#[derive(Clone, Copy)]
pub enum Topic {
    Faq,
    Manual,
    Controls,
    Support,
}
impl Topic {
    pub const ALL: [Self; 4] = [Self::Faq, Self::Manual, Self::Controls, Self::Support];
    pub fn label(self) -> &'static str {
        match self {
            Self::Faq => tr!("Frequently asked questions"),
            Self::Manual => tr!("Player manual"),
            Self::Controls => tr!("Client controls"),
            Self::Support => tr!("Help and reports"),
        }
    }
}
#[derive(Clone, Copy)]
pub enum Report {
    Bug,
    Player,
}

impl SettingsState {
    pub fn help_text(&self, topic: Topic) -> String {
        match topic {
            Topic::Faq => tr!("Frequently asked questions\r\n\r\nWhy was my action refused?\r\nThe server decides movement, combat and inventory results. Read its feedback and try a reachable target or a different item.\r\n\r\nWhere are my settings saved?\r\nSettings are local to this computer. Help / settings transfer lets you export them, import a saved file, restore a backup or restore defaults. A replacement saves the previous settings first.\r\n\r\nHow do I change shortcuts?\r\nOpen Controls, select an action and enter a new key combination. Conflicting bindings must be resolved before saving.\r\n\r\nWhy is a feature unavailable?\r\nSome panels require support from the current server session. Settings cannot enable an unavailable server feature.\r\n\r\nHow do I recover from a connection loss?\r\nReturn to sign-in and retry. Do not assume an item transfer or logout completed until the server confirms it.").into(),
            Topic::Controls => {
                let mut text = String::from(tr!("Current window shortcuts\r\n\r\n"));
                if let Ok(bindings) = preference_bindings() {
                    for (id, label) in WINDOW_ACTIONS {
            let label = oteryn_client_ui::localization::tr(label);
                        if let Some(binding) = bindings.iter().find(|binding| binding.id == *id) {
                            text.push_str(&format!("{label}: {}\r\n", self.preferences.binding_chord(binding)));
                        }
                    }
                }
                text.push_str(tr!("\r\nUse the Controls settings pages to edit shortcuts. Gameplay settings choose mouse behavior and chase cancellation. Screenshots chooses the destination, capture scope and automatic events."));
                text
            }
            Topic::Manual | Topic::Support => {
                let mut guide = Guide::default();
                guide.select(if matches!(topic, Topic::Manual) { Page::Basics } else { Page::Help });
                guide.text()
            }
        }
    }
    pub fn request_typed_help_report(&mut self, report: Report) {
        self.help_report = Some(report);
    }
    /// Run on the shell thread so native report windows retain their existing ownership.
    pub fn open_typed_help_report(&mut self, link: &crate::play::PlayLink) -> bool {
        let Some(report) = self.help_report.take() else {
            return false;
        };
        if !link.player_report_panel().selected {
            self.feedback = tr!("Reports are unavailable in this server session").into();
            return false;
        }
        match report {
            Report::Bug => link.bug_report_open(),
            Report::Player => link.player_report_open(),
        }
        true
    }
}
