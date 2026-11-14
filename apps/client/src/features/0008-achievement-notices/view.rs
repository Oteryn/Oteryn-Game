use crate::play::{PlayLink, mailbox::lock};
#[cfg(feature = "bevy-shell")]
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

impl PlayLink {
    pub fn achievement_notices_panel(
        &self,
    ) -> oteryn_client_ui::achievement_notices::AchievementNoticesPanel {
        lock(&self.mailbox).achievement_notices.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "bound_tests.rs"]
mod bound_tests;
/// A presentation snapshot tied to the exact session mailbox, never a second session owner.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct AchievementNoticesBinding {
    mailbox: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub(crate) panel: oteryn_client_ui::achievement_notices::AchievementNoticesPanel,
    used: Arc<AtomicBool>,
}
#[cfg(feature = "bevy-shell")]
impl AchievementNoticesBinding {
    pub(crate) fn same_projection(&self, other: &Self) -> bool {
        !self.used.load(Ordering::Relaxed)
            && self.mailbox.ptr_eq(&other.mailbox) && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn achievement_notices_binding(&self) -> AchievementNoticesBinding {
        AchievementNoticesBinding {
            mailbox: std::sync::Arc::downgrade(&self.mailbox),
            panel: self.achievement_notices_panel(),
            used: Arc::default(),
        }
    }

    /// Opening/closing is local presentation only. There is no query or mutation in this protocol.
    pub(crate) fn achievement_notices_set_open_bound(
        &self,
        expected: &AchievementNoticesBinding,
        open: bool,
    ) -> bool {
        if !expected.mailbox.ptr_eq(&std::sync::Arc::downgrade(&self.mailbox)) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.shared_frontend
            || mailbox.quest_shutdown
            // The retained terminal fence is set by end_custom_hotkeys on all session end paths.
            || mailbox.custom_hotkeys_ended
            || expected.used.load(Ordering::Relaxed)
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.achievement_notices != expected.panel
            || mailbox.achievement_notices.open == open
        {
            return false;
        }
        expected.used.store(true, Ordering::Relaxed);
        mailbox.achievement_notices.open = open;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_achievement_notices_snapshot(
        &self,
        selected: bool,
        watermark: Option<oteryn_client_ui::achievement_notices::AchievementWatermark>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.achievement_notices.selected = selected;
        mailbox.achievement_notices.watermark = watermark;
    }
    pub(crate) fn bevy_achievement_notices_earned(
        &self,
        earned: oteryn_client_ui::achievement_notices::AchievementEarned,
    ) {
        lock(&self.0).reduce_achievement_notices(vec![earned]);
    }
    pub(crate) fn bevy_achievement_notices_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
