//! Minimal Windows panel entry point; actions use the shared admitted command path.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::Intent;
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).cyclopedia_tracker.open = true;
    link.cyclopedia_tracker_request(Intent {
        command: 383,
        payload: Vec::new(),
    });
}
