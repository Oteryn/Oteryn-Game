//! Minimal native Browse Field panel.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).browse_field.open = true;
}
