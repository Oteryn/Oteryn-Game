use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel;
impl PlayLink {
    #[must_use]
    pub fn cyclopedia_character_panel(&self) -> CyclopediaCharacterPanel {
        lock(&self.mailbox).cyclopedia_character.clone()
    }
}
#[cfg(test)]
#[path = "native.rs"]
pub(in crate::play) mod native;

#[path = "view/blessings.rs"]
pub(super) mod blessings;
#[path = "view/sanctions.rs"]
pub(in crate::play) mod sanctions;
