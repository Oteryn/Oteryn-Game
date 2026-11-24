use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;
impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn equipped_weapon_progress(&self, observed: &mut Option<u32>) -> Option<u32> {
        let mut mailbox = lock(&self.mailbox);
        use oteryn_session::item_movement::EquipmentSlot;
        let weapon = mailbox.inventory.inventory.as_ref().and_then(|inventory| {
            let equipped = |slot| inventory.equipment.iter()
                .find(|entry| entry.slot == slot)
                .map(|entry| entry.item.item_definition_ref.get());
            let left = equipped(EquipmentSlot::LeftHand);
            let right = equipped(EquipmentSlot::RightHand);
            // A confirmed left-hand weapon can be paired with an off-hand item.
            left.filter(|id| mailbox.weapon_proficiency.catalogue.as_ref()
                .is_some_and(|page| page.entries.iter().any(|entry| entry.definition == *id)))
                .or(right)
                .or(left)
        });
        if !mailbox.weapon_proficiency.selected || mailbox.inventory_shutdown {
            *observed = None;
            return None;
        }
        let panel = &mut mailbox.weapon_proficiency;
        if *observed != weapon {
            panel.next_poll = None;
            *observed = weapon;
        }
        let weapon = weapon?;
        let progress = panel
            .catalogue
            .as_ref()
            .and_then(|page| page.entries.iter().find(|entry| entry.definition == weapon))
            .map(oteryn_client_ui::weapon_proficiency::progress_percent);
        let tick = !panel.open;
        drop(mailbox);
        if tick {
            self.weapon_proficiency_tick(std::time::Instant::now());
        }
        progress
    }
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
    Perk(usize, Option<usize>),
    /// Shaping slot 1 or 2 of the selected weapon.
    Slot(u8),
    /// One of the server's current reshape offers for the selected slot.
    Offer(u8),
    /// Modify the selected perk of this zero-based level in the selected slot.
    Modify(usize),
    /// Rank up, orb rank, reshape offer/choose/decline and the two-step clear.
    Shape(oteryn_client_ui::weapon_proficiency::Intent),
}
