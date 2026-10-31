use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    /// Authoritative chooser state rendered through the existing native shell status title.
    #[must_use]
    pub fn newhaven_vocation_status(&self) -> Option<&'static str> {
        let mailbox = lock(&self.mailbox);
        mailbox
            .vocation
            .open
            .then(|| oteryn_client_ui::newhaven_vocation::panel_text(&mailbox.vocation))
    }
}
