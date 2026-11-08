use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_encounter::BossEncounterPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_encounter_panel(&self) -> BossEncounterPanel {
        lock(&self.mailbox).boss_encounter.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{cooldown_window, open_window};
