use crate::play::{PlayLink, mailbox::lock};
pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).vip_presence.open = true;
}
