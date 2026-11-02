//! Minimal native Depot Search panel.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).depot_search.open = true;
}
