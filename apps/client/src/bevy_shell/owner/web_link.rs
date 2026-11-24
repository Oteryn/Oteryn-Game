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
        if self
            .web_link
            .as_ref()
            .is_some_and(|(epoch, _)| epoch.is_cancelled())
        {
            self.web_link = None;
            return;
        }
        if let Some(result) = self.web_link.as_ref().map(|(_, link)| link.try_recv())
            && result != Err(std::sync::mpsc::TryRecvError::Empty)
        {
            self.web_link = None;
            if result != Ok(true) {
                self.web_link_failed();
            }
        }
    }
}

#[cfg(test)]
#[test]
fn closed_web_link_result_preserves_current_login_feedback()
-> Result<(), Box<dyn std::error::Error>> {
    crate::account_services::test_server::test_thread(|| {
        let mut owner = HostOwner::new(None)?;
        let folder = tempfile::tempdir()?;
        let config = crate::reentry::Reentry::device_test_config(
            folder.path().into(),
            "https://localhost:1",
        )?;
        let entry = owner.entry.as_mut().ok_or("entry")?;
        assert!(entry.configure(config));
        assert!(entry.request_picker());
        owner.phase = Phase::Entering;
        owner.open_web_link(crate::web_links::WebLink::CookiePolicy);
        let epoch = owner.web_link.as_ref().ok_or("web link")?.0.clone();
        owner.close();
        assert!(epoch.is_cancelled());
        let message = owner.login.form.message().to_owned();
        owner.poll_web_link();
        assert!(owner.web_link.is_none());
        assert_eq!(owner.login.form.message(), message);
        Ok(())
    })
}

#[cfg(test)]
impl HostOwner {
    pub(crate) fn configure_test_web_link(&mut self, config: crate::NativeLoginConfig) -> bool {
        self.entry
            .as_mut()
            .is_some_and(|entry| entry.configure(config))
    }
}
