use super::*;
use std::sync::Arc;
use tokio::sync::watch;

pub(super) struct Revocation {
    config: Arc<crate::NativeLoginConfig>,
    result: watch::Receiver<Option<bool>>,
}

impl Revocation {
    fn start(
        config: Arc<crate::NativeLoginConfig>,
        runtime: &oteryn_client_runtime::ClientRuntime,
    ) -> Self {
        config.device_sessions.block_remembered();
        let (sender, result) = watch::channel(None);
        let held = Arc::clone(&config);
        let _task = runtime.spawn(async move {
            sender.send_replace(Some(held.forget_devices().await));
        });
        Self { config, result }
    }

    fn finished(&self) -> Option<bool> {
        let result = *self.result.borrow();
        result.or_else(|| {
            self.result
                .has_changed()
                .is_err()
                .then(|| self.result.borrow().unwrap_or(false))
        })
    }

    pub(super) fn pending(&self) -> bool {
        self.finished().is_none()
    }

    pub(super) fn matches(&self, config: &crate::NativeLoginConfig) -> bool {
        self.config.platform_url == config.platform_url
            && self.config.oauth_client_id == config.oauth_client_id
    }

    pub(super) fn reuse_device(&self, config: &mut crate::NativeLoginConfig) {
        config.device_sessions = self.config.device_sessions.clone();
    }
}

impl Reentry {
    /// Cancel just this attempt, preserving the process runtime for a fresh sign-in.
    pub fn cancel_entry(&mut self) -> bool {
        let Some(token) = &self.attempt_cancellation else {
            return false;
        };
        if self.state != EntryState::Entering || token.is_cancelled() {
            return false;
        }
        {
            let _delivery = self
                .browser_delivery
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            token.cancel();
        }
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

    pub fn native_sign_out(&mut self) -> Option<watch::Receiver<Option<bool>>> {
        if let Some(config) = &self.client.native_login {
            config.training_queue.clear();
        }
        self.cancel_entry();
        self.forget_native_session()
    }
    /// Explicit native sign-out; normal game logout keeps a remembered login available.
    pub fn forget_native_session(&mut self) -> Option<watch::Receiver<Option<bool>>> {
        let config = self.client.native_login.as_ref()?;
        let runtime = &self.client.runtime;
        if let Some(previous) = self
            .revocations
            .iter_mut()
            .find(|previous| previous.matches(config))
        {
            if previous.finished().is_some() {
                *previous = Revocation::start(Arc::new(config.clone()), runtime);
            }
            return Some(previous.result.clone());
        }
        let revocation = Revocation::start(Arc::new(config.clone()), runtime);
        let result = revocation.result.clone();
        self.revocations.push(revocation);
        Some(result)
    }

    pub(super) fn revocation_for_entry(&mut self) -> Option<watch::Receiver<Option<bool>>> {
        self.revocations
            .retain(|revocation| revocation.finished() != Some(true));
        let config = self.client.native_login.as_ref()?;
        self.revocations
            .iter()
            .find(|revocation| revocation.matches(config) && revocation.pending())
            .map(|revocation| revocation.result.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn failed_platform_revocation_does_not_fence_fresh_admission()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = |origin: &str| {
            crate::NativeLoginConfig::from_env(|name| {
                Some(match name {
                    "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => origin.into(),
                    "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
                    "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
                    _ => String::new(),
                })
            })
        };
        let first = Arc::new(config("https://localhost:1")?.ok_or("first Platform")?);
        let second = config("https://localhost:2")?.ok_or("second Platform")?;
        let mut entry = Reentry::new(ClientBootstrap::new()?);
        assert!(entry.configure((*first).clone()));
        let (sender, result) = watch::channel(None);
        entry.revocations.push(Revocation {
            config: first.clone(),
            result,
        });
        assert!(entry.revocation_for_entry().is_some());
        assert!(entry.configure(second));
        assert!(entry.revocation_for_entry().is_none());
        sender.send_replace(Some(false));
        assert!(entry.configure((*first).clone()));
        assert!(entry.revocation_for_entry().is_none());
        entry.shutdown();
        Ok(())
    }

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
    assert!(entry.poll().is_none());
    assert!(!entry.request());
    sender.send(Err(GameplayEntryError::Rejected(
        PublicClass::SessionUnavailable,
    )))?;
    assert!(matches!(entry.poll(), Some(Err(_))));
    drop(held_session);
    entry.shutdown();
    Ok(())
}

#[cfg(test)]
#[test]
fn poll_consumes_cancelled_attempt_while_revocation_is_pending()
-> Result<(), Box<dyn std::error::Error>> {
    let mut entry = Reentry::new(ClientBootstrap::new()?);
    let (sender, receiver) = mpsc::channel();
    entry.pending = Some(receiver);
    entry.state = EntryState::Entering;
    entry.attempt_cancellation = Some(CancellationToken::new());
    let config = crate::NativeLoginConfig::from_env(|name| {
        Some(match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => "https://localhost:1".into(),
            "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
            "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
            _ => String::new(),
        })
    })?
    .ok_or("native login config")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let device = config.device_sessions.clone();
    let held = runtime.block_on(device.hold_session_for_reentry_test());
    entry.client.native_login = Some(config);
    entry.forget_native_session();
    assert!(entry.cancel_entry());
    sender.send(Err(GameplayEntryError::Rejected(
        PublicClass::SessionUnavailable,
    )))?;
    assert!(matches!(entry.poll(), Some(Err(_))));
    assert!(!entry.is_entering());
    drop(held);
    entry.shutdown();
    Ok(())
}
