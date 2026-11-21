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
    /// Shaping slot 1 or 2 of the selected weapon.
    Slot(u8),
    /// One of the server's current reshape offers for the selected slot.
    Offer(u8),
    /// Modify the selected perk of this zero-based level in the selected slot.
    Modify(usize),
    /// Rank up, orb rank, reshape offer/choose/decline and the two-step clear.
    Shape(oteryn_client_ui::weapon_proficiency::Intent),
}
