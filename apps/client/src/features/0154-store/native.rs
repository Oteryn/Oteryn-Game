//! Minimal native Store panel.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).store.open = true;
    link.discovery_open(285);
}
