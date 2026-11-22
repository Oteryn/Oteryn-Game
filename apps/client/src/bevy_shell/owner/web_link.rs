use super::{HostOwner, Phase};

impl HostOwner {
    /// Opens a Platform page (support tickets, cookie policy) from the login window with the
    /// Platform the connection settings name.
    pub fn open_web_link(&mut self, link: crate::web_links::WebLink) {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            entry.configure(config);
        }
        self.web_link = self
            .entry
            .as_ref()
            .and_then(|entry| entry.start_web_link(link));
        if self.web_link.is_none() {
            self.web_link_failed();
        }
    }
    fn web_link_failed(&mut self) {
        self.login.form.feedback(
            "Correct the connection settings to open Platform pages in the browser.".into(),
        );
    }
    pub(super) fn poll_web_link(&mut self) {
        if let Some(result) = self.web_link.as_ref().map(|link| link.try_recv())
            && result != Err(std::sync::mpsc::TryRecvError::Empty)
        {
            self.web_link = None;
            if result != Ok(true) {
                self.web_link_failed();
            }
        }
    }
}
