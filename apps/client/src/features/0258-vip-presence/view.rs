use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn vip_presence_panel(&self) -> oteryn_client_ui::vip_presence::VipPresencePanel {
        let mailbox = lock(&self.mailbox);
        let mut panel = mailbox.vip_presence.clone();
        if let Some(confirmed_at) = mailbox.vip_presence_confirmed_at {
            oteryn_session::expire_vip_presence(&mut panel.presence, confirmed_at.elapsed());
        }
        panel
    }
}

impl crate::play::PlayView {
    /// chsw (capability 513): show each VIP login, logout or Channel change once.
    pub(in crate::play) fn present_vip_channel_notices(&self, link: &PlayLink) {
        let notices = std::mem::take(&mut lock(&link.mailbox).vip_channel_notices);
        for (serial, text) in notices {
            self.game_notifications
                .show(&format!("vip-channel:{serial}"), &text, false);
        }
    }
}
