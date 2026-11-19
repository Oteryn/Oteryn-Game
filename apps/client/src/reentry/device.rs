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
        if let Some(config) = &self.client.native_login {
            config.training_queue.clear();
        }
        if let Some((_, queue)) = &mut self.waiting {
            queue.cancel();
        }
        self.last_wait_text = None;
        true
    }

    #[cfg(all(test, bevy_shell))]
    pub(crate) fn arm_cancel_fixture(&mut self) {
        self.state = EntryState::Entering;
        self.attempt_cancellation = Some(CancellationToken::new());
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
        self.cancel_entry();
        self.forget_native_session();
    }
    /// Explicit native sign-out; normal game logout keeps a remembered login available.
    pub fn forget_native_session(&mut self) {
        if let Some(config) = self.client.native_login.clone() {
            let previous = self.revocation.take();
            self.revocation = self
                .client
                .runtime
                .spawn(async move {
                    if let Some(previous) = previous {
                        let _ = previous.await;
                    }
                    config.forget_devices().await;
                })
                .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn cancellation_preserves_device_and_explicit_sign_out_schedules_revocation()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let config = crate::NativeLoginConfig::from_env(|name| {
            Some(match name {
                "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => "https://localhost:1".into(),
                "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
                "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
                _ => String::new(),
            })
        })?
        .ok_or("native login config")?;
        let device = Arc::clone(&config.device_sessions);
        // Keep a revocation task alive if cancellation wrongly schedules one.
        let held_session = runtime.block_on(device.hold_session_for_reentry_test());
        let mut entry = Reentry::new(ClientBootstrap::new()?);
        assert!(entry.configure(config));
        entry.state = EntryState::Entering;
        entry.attempt_cancellation = Some(CancellationToken::new());
        let owners = Arc::strong_count(&device);
        assert!(entry.cancel_entry());
        assert_eq!(
            Arc::strong_count(&device),
            owners,
            "attempt cancellation must not launch device revocation"
        );
        entry.attempt_cancellation = Some(CancellationToken::new());
        entry.native_sign_out();
        assert_eq!(
            Arc::strong_count(&device),
            owners + 1,
            "explicit sign-out must still launch revocation during admission"
        );
        drop(held_session);
        entry.shutdown();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn cancelled_admission_cannot_be_retried_until_device_revocation_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let config = crate::NativeLoginConfig::from_env(|name| {
        Some(match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => "https://localhost:1".into(),
            "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
            "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
            _ => String::new(),
        })
    })?
    .ok_or("native login config")?;
    let device = config.device_sessions.clone();
    let held_session = runtime.block_on(device.hold_session_for_reentry_test());
    let mut entry = Reentry::new(ClientBootstrap::new()?);
    assert!(entry.configure(config));
    let (sender, receiver) = mpsc::channel();
    entry.pending = Some(receiver);
    entry.state = EntryState::Entering;
    entry.attempt_cancellation = Some(CancellationToken::new());
    assert!(entry.cancel_entry());
    entry.forget_native_session();
    sender.send(Err(GameplayEntryError::Rejected(
        PublicClass::SessionUnavailable,
    )))?;
    assert!(entry.poll().is_none());
    assert!(!entry.request());
    drop(held_session);
    entry.shutdown();
    Ok(())
}
