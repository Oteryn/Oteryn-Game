use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;
impl PlayLink {
    pub fn weapon_proficiency_is_open(&self) -> bool {
        lock(&self.mailbox).weapon_proficiency.open
    }
    pub fn weapon_proficiency_hud(&self) -> Option<oteryn_client_ui::weapon_proficiency::HudView> {
        oteryn_client_ui::weapon_proficiency::hud_view(&lock(&self.mailbox).weapon_proficiency)
    }
    #[must_use]
    pub fn weapon_proficiency_panel(&self) -> WeaponProficiencyPanel {
        lock(&self.mailbox).weapon_proficiency.clone()
    }
}
#[cfg(test)]
#[path = "native.rs"]
pub(in crate::play) mod native;

/// Actions from a presentation bound to the exact observed catalogue.
#[derive(Clone, Copy)]
pub enum ProficiencyAction {
    Close,
    Refresh,
    Weapon(usize),
    Perk(usize, usize),
}
