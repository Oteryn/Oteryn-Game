//! Minimal native Item Forge panel.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).item_forge.open = true;
}
