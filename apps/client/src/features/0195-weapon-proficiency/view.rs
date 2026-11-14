use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;
impl PlayLink {
    pub fn weapon_proficiency_hud(&self) -> Option<oteryn_client_ui::weapon_proficiency::HudView> {
        oteryn_client_ui::weapon_proficiency::hud_view(&lock(&self.mailbox).weapon_proficiency)
    }
    #[must_use]
    pub fn weapon_proficiency_panel(&self) -> WeaponProficiencyPanel {
        lock(&self.mailbox).weapon_proficiency.clone()
    }
}
#[cfg(any(windows, test, feature = "bevy-shell"))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
pub(crate) mod bound;
