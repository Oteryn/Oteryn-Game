//! Minimal native VIP controls; authoritative state remains rendered as the server view.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip::Intent;

pub(in crate::play) fn open_window(link: &PlayLink) {
    lock(&link.mailbox).vip.open = true;
    // The native shell's list and action buttons use the same public action path. Keep the
    // initial action explicit so opening always refreshes the authoritative list.
    link.vip_request(Intent::Query(0));
}
