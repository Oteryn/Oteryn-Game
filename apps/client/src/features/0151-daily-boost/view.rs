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
