use super::*;
impl Reentry {
    /// Cancel just this attempt, preserving the process runtime for a fresh sign-in.
    pub fn cancel_entry(&mut self) -> bool {
        let Some(token) = &self.attempt_cancellation else {
            return false;
        };
        if self.state != EntryState::Entering || token.is_cancelled() {
            return false;
        }
        token.cancel();
        self.forget_native_session();
        if let Some(config) = &self.client.native_login {
            config.training_queue.clear();
        }
        if let Some((_, queue)) = &mut self.waiting {
            queue.cancel();
        }
        self.last_wait_text = None;
        true
    }

    /// Opens a Platform web page in the system browser; `false` without native login.
    pub fn open_web_link(&self, link: crate::web_links::WebLink) -> bool {
        self.client
            .native_login
            .as_ref()
            .is_some_and(|config| crate::web_links::open(config, link))
    }

    pub fn native_sign_out(&mut self) {
        if let Some(config) = &self.client.native_login {
            config.training_queue.clear();
        }
        if !self.cancel_entry() {
            self.forget_native_session();
        }
    }
    /// Explicit native sign-out; normal game logout keeps a remembered login available.
    pub fn forget_native_session(&self) {
        if let Some(config) = self.client.native_login.clone() {
            let _ = self.client.runtime.spawn(async move {
                config.forget_devices().await;
            });
        }
    }
}
