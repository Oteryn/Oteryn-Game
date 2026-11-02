//! Minimal native Hotkey Equip panel.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).hotkey_equip.open = true;
}
