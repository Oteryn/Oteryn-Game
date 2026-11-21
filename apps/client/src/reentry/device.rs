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
    pub(crate) fn can_cancel_entry(&self) -> bool {
        self.state == EntryState::Entering
            && self
                .attempt_cancellation
                .as_ref()
                .is_some_and(|token| !token.is_cancelled())
    }

    /// Cancel just this attempt, preserving the process runtime for a fresh sign-in.
    pub fn cancel_entry(&mut self) -> bool {
        if !self.can_cancel_entry() {
            return false;
        }
        if let Some(token) = &self.attempt_cancellation {
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

    #[cfg(test)]
    pub(crate) fn device_test_config(
        folder: std::path::PathBuf,
        origin: &str,
    ) -> Result<crate::NativeLoginConfig, Box<dyn std::error::Error>> {
        let mut config = crate::NativeLoginConfig::from_env(|name| {
            Some(match name {
                "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => origin.into(),
                "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
                "OTERYN_DEV_ROOT" => folder.join("unused.der").to_string_lossy().into_owned(),
                _ => String::new(),
            })
        })?
        .ok_or("native login config")?;
        config.device_sessions = Arc::new(crate::device_login::DeviceLogin::test_fixture(folder));
        config.open_browser = Arc::new(|_| false);
        Ok(config)
    }

    /// Opens a Platform web page in the system browser; `false` without native login.
    pub fn open_web_link(&self, link: crate::web_links::WebLink) -> bool {
        self.client
            .native_login
            .as_ref()
            .is_some_and(|config| crate::web_links::open(config, link))
    }

    pub fn native_sign_out(
        &mut self,
    ) -> Result<
        Option<watch::Receiver<Option<bool>>>,
        oteryn_platform_client::native_login::NativeLoginError,
    > {
        let result = self.forget_native_session()?;
        if let Some(config) = &self.client.native_login {
            config.training_queue.clear();
        }
        self.cancel_entry();
        Ok(result)
    }
    /// Explicit native sign-out; normal game logout keeps a remembered login available.
    pub fn forget_native_session(
        &mut self,
    ) -> Result<
        Option<watch::Receiver<Option<bool>>>,
        oteryn_platform_client::native_login::NativeLoginError,
    > {
        let Some(config) = self.client.native_login.as_mut() else {
            return Ok(None);
        };
        config.device_sessions.block_remembered()?;
        let options = config.session_options.get_or_insert_with(|| {
            config
                .preference_path
                .as_deref()
                .and_then(|path| {
                    crate::play::MiscellaneousPreferences::read(
                        &path.with_extension("session-options.json"),
                    )
                    .ok()
                })
                .unwrap_or_default()
        });
        options.remember_device = false;
        let runtime = &self.client.runtime;
        if let Some(previous) = self
            .revocations
            .iter_mut()
            .find(|previous| previous.matches(config))
        {
            if previous.finished().is_some() {
                *previous = Revocation::start(Arc::new(config.clone()), runtime);
            }
            return Ok(Some(previous.result.clone()));
        }
        let revocation = Revocation::start(Arc::new(config.clone()), runtime);
        let result = revocation.result.clone();
        self.revocations.push(revocation);
        Ok(Some(result))
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
        let fixture = tempfile::tempdir()?;
        let first = Arc::new(Reentry::device_test_config(
            fixture.path().join("first"),
            "https://localhost:1",
        )?);
        let second =
            Reentry::device_test_config(fixture.path().join("second"), "https://localhost:2")?;
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
        let fixture = tempfile::tempdir()?;
        let config =
            Reentry::device_test_config(fixture.path().join("device"), "https://localhost:1")?;
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
        let revocation = entry.native_sign_out()?.ok_or("queued revocation")?;
        assert_eq!(
            Arc::strong_count(&device),
            owners + 1,
            "explicit sign-out must still launch revocation during admission"
        );
        drop(held_session);
        assert!(wait_for_revocation(&runtime, revocation)?);
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
    let fixture = tempfile::tempdir()?;
    let folder = fixture.path().join("device");
    let mut config = Reentry::device_test_config(folder.clone(), "https://localhost:1")?;
    let (browser_opened, observed) = mpsc::channel();
    config.open_browser = Arc::new(move |_| {
        let _ = browser_opened.send(!folder.join("revoke.pending").exists());
        false
    });
    let device = config.device_sessions.clone();
    let held_session = runtime.block_on(device.hold_session_for_reentry_test());
    let mut entry = Reentry::new(ClientBootstrap::new()?);
    assert!(entry.configure(config));
    let (sender, receiver) = mpsc::channel();
    entry.pending = Some(receiver);
    entry.state = EntryState::Entering;
    entry.attempt_cancellation = Some(CancellationToken::new());
    assert!(entry.cancel_entry());
    let revocation = entry.forget_native_session()?.ok_or("queued revocation")?;
    assert!(entry.poll().is_none());
    assert!(!entry.request());
    sender.send(Err(GameplayEntryError::Rejected(
        PublicClass::SessionUnavailable,
    )))?;
    assert!(matches!(entry.poll(), Some(Err(_))));
    assert!(entry.revocation_for_entry().is_some());
    assert!(
        entry.request(),
        "a retry after cancellation must enter the revocation gate"
    );
    drop(held_session);
    assert!(wait_for_revocation(&runtime, revocation)?);
    assert!(entry.revocation_for_entry().is_none());
    assert!(observed.recv_timeout(std::time::Duration::from_secs(2))?);
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
    let fixture = tempfile::tempdir()?;
    let config = Reentry::device_test_config(fixture.path().join("device"), "https://localhost:1")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let device = config.device_sessions.clone();
    let held = runtime.block_on(device.hold_session_for_reentry_test());
    entry.client.native_login = Some(config);
    let revocation = entry.forget_native_session()?.ok_or("queued revocation")?;
    assert!(entry.cancel_entry());
    sender.send(Err(GameplayEntryError::Rejected(
        PublicClass::SessionUnavailable,
    )))?;
    assert!(matches!(entry.poll(), Some(Err(_))));
    assert!(!entry.is_entering());
    drop(held);
    assert!(wait_for_revocation(&runtime, revocation)?);
    entry.shutdown();
    Ok(())
}

#[cfg(test)]
fn wait_for_revocation(
    runtime: &tokio::runtime::Runtime,
    mut revocation: watch::Receiver<Option<bool>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    runtime.block_on(async {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            revocation.wait_for(Option::is_some),
        )
        .await??;
        result.ok_or_else(|| "revocation result missing".into())
    })
}
