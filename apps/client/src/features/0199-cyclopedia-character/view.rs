use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel;
impl PlayLink {
    #[must_use]
    pub fn cyclopedia_character_panel(&self) -> CyclopediaCharacterPanel {
        lock(&self.mailbox).cyclopedia_character.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
