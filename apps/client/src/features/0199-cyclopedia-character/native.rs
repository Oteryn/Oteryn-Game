//! Minimal Windows panel entry point; actions use the shared admitted command path.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::Intent;
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).cyclopedia_character.open = true;
    link.cyclopedia_character_request(Intent {
        command: 387,
        payload: Vec::new(),
    });
}
