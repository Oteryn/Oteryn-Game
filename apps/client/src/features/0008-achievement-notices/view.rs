use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    /// Consume only transient Bevy presentation events; retained panel history is unchanged.
    #[cfg(bevy_shell)]
    pub(crate) fn take_achievement_banners(&self) -> std::collections::VecDeque<String> {
        std::mem::take(&mut lock(&self.mailbox).achievement_banners)
    }

    pub fn achievement_notices_panel(
        &self,
    ) -> oteryn_client_ui::achievement_notices::AchievementNoticesPanel {
        lock(&self.mailbox).achievement_notices.clone()
    }
}

/// The shared frontend's read-only notice list; the window closes like the native one.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Achievement Notices — Ctrl+Shift+M"),
        labels: &[],
        buttons: &[],
        snapshot: |mailbox| {
            (
                mailbox.achievement_notices.open && !mailbox.quest_shutdown,
                false,
                Vec::new(),
                oteryn_client_ui::achievement_notices::notices_panel_text(
                    &mailbox.achievement_notices,
                ),
            )
        },
        set_open: |mailbox, value| mailbox.achievement_notices.open = value,
        submit: |_, _, _, _| {},
    }
}
