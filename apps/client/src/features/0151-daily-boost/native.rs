//! Minimal Windows panel entry point; actions use the shared admitted command path.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::Intent;
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).daily_boost.open = true;
    link.daily_boost_request(Intent {
        command: 177,
        payload: Vec::new(),
    });
}
