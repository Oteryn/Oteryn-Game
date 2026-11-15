use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;
impl PlayLink {
    pub fn weapon_proficiency_is_open(&self) -> bool {
        lock(&self.mailbox).weapon_proficiency.open
    }
    pub fn weapon_proficiency_hud(&self) -> Option<oteryn_client_ui::weapon_proficiency::HudView> {
        let mailbox = lock(&self.mailbox);
        let definition = equipped_weapon(&mailbox)?;
        oteryn_client_ui::weapon_proficiency::hud_view_for(&mailbox.weapon_proficiency, definition)
    }
    #[must_use]
    pub fn weapon_proficiency_panel(&self) -> WeaponProficiencyPanel {
        lock(&self.mailbox).weapon_proficiency.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// Actions from a presentation bound to the exact observed catalogue.
#[derive(Clone, Copy)]
pub enum ProficiencyAction {
    Close,
    Refresh,
    Weapon(usize),
    Perk(usize, usize),
}

pub(in crate::play) fn equipped_weapon(mailbox: &crate::play::mailbox::Mailbox) -> Option<u32> {
    use oteryn_session::item_view::EquipmentSlot;
    let entries = &mailbox.weapon_proficiency.catalogue.as_ref()?.entries;
    mailbox
        .inventory
        .inventory
        .as_ref()?
        .equipment
        .iter()
        .filter(|item| {
            matches!(
                item.slot,
                EquipmentSlot::RightHand | EquipmentSlot::LeftHand
            )
        })
        .map(|item| item.item.item_definition_ref.get())
        .find(|definition| entries.iter().any(|entry| entry.definition == *definition))
}
