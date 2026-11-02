//! Minimal Windows panel entry point; actions use the shared admitted command path.
use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).weapon_proficiency.open = true;
}
