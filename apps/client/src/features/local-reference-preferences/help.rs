//! Settings navigation reuses the Oteryn player guide and the reviewed report callers.
use super::{SettingsState, WINDOW_ACTIONS, preference_bindings};
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
            Self::Faq => "Frequently asked questions",
            Self::Manual => "Player manual",
            Self::Controls => "Client controls",
            Self::Support => "Help and reports",
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
            Topic::Faq => "Frequently asked questions\r\n\r\nWhy was my action refused?\r\nThe server decides movement, combat and inventory results. Read its feedback and try a reachable target or a different item.\r\n\r\nWhere are my settings saved?\r\nSettings are local to this computer. Help / settings transfer lets you export them, import a saved file, restore a backup or restore defaults. A replacement saves the previous settings first.\r\n\r\nHow do I change shortcuts?\r\nOpen Controls, select an action and enter a new key combination. Conflicting bindings must be resolved before saving.\r\n\r\nWhy is a feature unavailable?\r\nSome panels require support from the current server session. Settings cannot enable an unavailable server feature.\r\n\r\nHow do I recover from a connection loss?\r\nReturn to sign-in and retry. Do not assume an item transfer or logout completed until the server confirms it.".into(),
            Topic::Controls => {
                let mut text = String::from("Current window shortcuts\r\n\r\n");
                if let Ok(bindings) = preference_bindings() {
                    for (id, label) in WINDOW_ACTIONS {
                        if let Some(binding) = bindings.iter().find(|binding| binding.id == *id) {
                            text.push_str(&format!("{label}: {}\r\n", self.preferences.binding_chord(binding)));
                        }
                    }
                }
                text.push_str("\r\nUse the Controls settings pages to edit shortcuts. Gameplay settings choose mouse behavior and chase cancellation. Screenshots chooses the destination, capture scope and automatic events.");
                text
            }
            Topic::Manual | Topic::Support => {
                let mut guide = Guide::default();
                guide.select(if matches!(topic, Topic::Manual) { Page::Basics } else { Page::Help });
                guide.text()
            }
        }
    }
    pub fn request_help_report(&mut self, report: Report) {
        self.help_report = Some(report);
    }
    /// Run on the shell thread so native report windows retain their existing ownership.
    pub fn open_help_report(&mut self, link: &crate::play::PlayLink) -> bool {
        let Some(report) = self.help_report.take() else {
            return false;
        };
        if !link.player_report_panel().selected {
            self.feedback = "Reports are unavailable in this server session".into();
            return false;
        }
        match report {
            Report::Bug => link.bug_report_open(),
            Report::Player => link.player_report_open(),
        }
        true
    }
}
