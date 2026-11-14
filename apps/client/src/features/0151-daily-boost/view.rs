use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::DailyBoostPanel;
impl PlayLink {
    #[must_use]
    pub fn daily_boost_panel(&self) -> DailyBoostPanel {
        lock(&self.mailbox).daily_boost.clone()
    }
}
#[cfg(all(windows, not(test)))]
#[path = "native.rs"]
mod native;
#[cfg(all(windows, not(test)))]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_daily_boost_state(&self, panel: DailyBoostPanel) {
        lock(&self.0).daily_boost = panel;
    }
}

#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct DailyBoostBinding {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub(crate) panel: DailyBoostPanel,
    pub(crate) xp: String,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn daily_boost_binding(&self) -> DailyBoostBinding {
        let mailbox = lock(&self.mailbox);
        let durations = &mailbox.potion_durations;
        let xp = durations
            .snapshot
            .as_ref()
            .map_or_else(String::new, |snapshot| {
                let elapsed = durations.received_at.map_or(0, |at| {
                    u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
                });
                oteryn_client_ui::daily_boost::xp_boost_lines(&snapshot.active, elapsed)
            });
        DailyBoostBinding {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: mailbox.daily_boost.clone(),
            xp,
        }
    }
    pub(crate) fn daily_boost_close_bound(&self, expected: &DailyBoostBinding) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.daily_boost != expected.panel
            || !mailbox.daily_boost.open
        {
            return false;
        }
        mailbox.daily_boost.open = false;
        true
    }
}

#[cfg(feature = "bevy-shell")]
impl DailyBoostBinding {
    pub(crate) fn same_projection(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel && self.xp == other.xp
    }
}
