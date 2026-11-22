use super::*;

enum ForgetAction {
    CancelLogin,
    CancelPicker(u64),
    SignOut { configured: bool },
    Options(Box<Draft>, bool),
}

pub(super) struct PendingIntent {
    worker: std::thread::JoinHandle<Result<(), String>>,
    action: ForgetAction,
    scope: Scope,
    config: crate::NativeLoginConfig,
}

impl HostOwner {
    pub fn cancel_login(&mut self) {
        if self.entry.as_ref().is_some_and(Reentry::can_cancel_entry) {
            self.queue_forget(ForgetAction::CancelLogin);
        } else {
            self.cancel_login_ui();
        }
    }

    fn cancel_login_ui(&mut self) {
        self.login.cancel_save();
        self.selection.cancel();
        self.services.cancel();
        if let Some(entry) = &mut self.entry {
            entry.cancel_entry();
        }
    }

    pub(in crate::bevy_shell) fn cancel_picker(&mut self, id: u64) {
        self.queue_forget(ForgetAction::CancelPicker(id));
    }

    pub fn sign_out_device(&mut self) {
        if self.play.is_none() && self.forget_intent.is_none() {
            self.login.cancel_save();
            let configured = self.configure_offline_forget();
            self.queue_forget(ForgetAction::SignOut { configured });
        }
    }

    pub(super) fn forget_for_options(&mut self, draft: Draft, close: bool) {
        self.configure_offline_forget();
        self.queue_forget(ForgetAction::Options(Box::new(draft), close));
    }

    fn configure_offline_forget(&mut self) -> bool {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            return entry.configure(config);
        }
        false
    }

    fn queue_forget(&mut self, action: ForgetAction) {
        if self.forget_intent.is_some() {
            return;
        }
        let Some(config) = self
            .entry
            .as_ref()
            .and_then(Reentry::config_for_forget)
            .cloned()
        else {
            match self.settings.disable_remember_device() {
                Ok(()) => self.apply_forget(action),
                Err(error) => self.forget_failed(action, error),
            }
            return;
        };
        let worker_config = config.clone();
        let picker = matches!(action, ForgetAction::CancelPicker(_));
        match std::thread::Builder::new()
            .name("device-forget".into())
            .spawn(move || {
                if picker {
                    worker_config.record_picker_opt_out()
                } else {
                    worker_config
                        .device_sessions
                        .block_remembered()
                        .map_err(|error| error.to_string())
                }
            }) {
            Ok(worker) => {
                self.selection.pause_responses(true);
                self.forget_intent = Some(PendingIntent {
                    worker,
                    action,
                    config,
                    scope: self.scope,
                })
            }
            Err(error) => self.forget_failed(action, error.to_string()),
        }
    }

    fn forget_failed(&mut self, action: ForgetAction, error: String) {
        if let ForgetAction::Options(draft, _) = action {
            self.draft = Some(*draft);
        }
        let message = format!("Could not record device sign-out: {error}");
        self.settings.feedback.clone_from(&message);
        self.login.form.feedback(message);
    }

    pub(super) fn forget_worker_running(&self) -> bool {
        self.forget_intent
            .as_ref()
            .is_some_and(|intent| !intent.worker.is_finished())
    }

    pub(super) fn finish_forget_intent(&mut self) {
        if let Some(intent) = self.forget_intent.take() {
            let result = intent
                .worker
                .join()
                .unwrap_or_else(|_| Err("Device writer stopped".into()));
            self.selection.pause_responses(false);
            if let Err(error) = result.and_then(|()| self.settings.disable_remember_device()) {
                self.forget_failed(intent.action, error);
            } else {
                let current = self
                    .entry
                    .as_ref()
                    .and_then(Reentry::config_for_forget)
                    .is_some_and(|config| {
                        std::sync::Arc::ptr_eq(
                            &config.device_sessions,
                            &intent.config.device_sessions,
                        )
                    });
                if let Some(entry) = &mut self.entry {
                    entry.set_session_options(self.settings.preferences.miscellaneous);
                    self.device_sign_out = Some(entry.finish_recorded_forget(intent.config));
                }
                if current && self.scope == intent.scope {
                    self.apply_forget(intent.action);
                }
            }
        }
    }

    fn apply_forget(&mut self, action: ForgetAction) {
        if self.phase == Phase::Closed {
            return;
        }
        match action {
            ForgetAction::CancelLogin => self.cancel_login_ui(),
            ForgetAction::CancelPicker(id) => {
                self.selection
                    .respond(id, crate::character_selection::SelectionAction::Cancel);
            }
            ForgetAction::SignOut { configured } => {
                if let Some(entry) = &mut self.entry {
                    entry.cancel_after_sign_out();
                }
                self.selection.cancel();
                self.services.cancel();
                self.login.form.feedback(if configured && self.device_sign_out.is_some() {
                    "Signing out this device…".into()
                } else {
                    "Remember-device is off. Correct the connection settings to also forget this device on the Platform.".into()
                });
            }
            ForgetAction::Options(mut draft, close) => {
                if let Some(entry) = &mut self.entry {
                    entry.cancel_after_sign_out();
                }
                if let Some(play) = &mut self.play {
                    play.view.logout();
                }
                draft.values.miscellaneous.remember_device = false;
                self.apply_draft(*draft, close);
            }
        }
    }
    pub(super) fn poll_device_sign_out(&mut self) {
        if self
            .forget_intent
            .as_ref()
            .is_some_and(|intent| intent.worker.is_finished())
        {
            self.finish_forget_intent();
        }
        if let Some(result) = self.device_sign_out.as_ref().and_then(|receiver| {
            (*receiver.borrow()).or_else(|| {
                receiver
                    .has_changed()
                    .is_err()
                    .then(|| receiver.borrow().unwrap_or(false))
            })
        }) {
            self.device_sign_out = None;
            self.login.form.feedback(if result {
                "Signed out; this device is no longer remembered.".into()
            } else {
                "Could not finish signing out this device. Try signing out again.".into()
            });
        }
    }
}

#[cfg(test)]
#[test]
fn cancel_login_schedules_remembered_device_revocation() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let fixture = tempfile::tempdir()?;
    let folder = fixture.path().join("device");
    let path = fixture.path().join("preferences.json");
    let config = Reentry::device_test_config(folder.clone(), "https://localhost:1")?;
    let device = Arc::clone(&config.device_sessions);
    let held_session = runtime.block_on(device.hold_session_for_reentry_test());
    let mut owner = HostOwner::new(Some(path.clone()))?;
    let options = crate::play::MiscellaneousPreferences {
        remember_device: true,
        ..Default::default()
    };
    options.save(&path.with_extension("session-options.json"))?;
    let entry = owner.entry.as_mut().ok_or("reentry")?;
    assert!(entry.configure(config));
    entry.arm_cancel_fixture();
    owner.phase = Phase::Entering;
    let owners = Arc::strong_count(&device);
    owner.settings.preferences.miscellaneous.remember_device = true;
    owner.cancel_login();
    owner.finish_forget_intent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while owner.settings.local_preference_writes_pending() {
        assert!(std::time::Instant::now() < deadline);
        assert!(owner.settings.poll_background_save().error.is_none());
    }
    assert_eq!(Arc::strong_count(&device), owners + 1);
    assert!(!owner.settings.preferences.miscellaneous.remember_device);

    assert!(
        !crate::play::MiscellaneousPreferences::read(&path.with_extension("session-options.json"))?
            .remember_device
    );
    owner.close();
    assert!(folder.join("revoke.pending").is_file());
    drop(held_session);
    Ok(())
}

#[cfg(test)]
#[test]
fn review_3376_cancel_preserves_entry_when_intent_cannot_be_written()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let device = folder.path().join("device");
    std::fs::create_dir_all(device.join("revoke.pending"))?;
    let config = Reentry::device_test_config(device, "https://localhost:1")?;
    let mut owner = HostOwner::new(None)?;
    let entry = owner.entry.as_mut().ok_or("entry")?;
    assert!(entry.configure(config));
    entry.arm_cancel_fixture();
    owner.settings.preferences.miscellaneous.remember_device = true;
    owner.cancel_login();
    owner.finish_forget_intent();
    assert!(
        owner.entry.as_mut().ok_or("entry")?.cancel_entry(),
        "the unsuccessful forget must leave the attempt active"
    );
    assert!(owner.settings.preferences.miscellaneous.remember_device);
    Ok(())
}

#[cfg(test)]
#[test]
fn c2d_process_forget_stages_vault_off_frame() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let mut owner = HostOwner::new(None)?;
    let mut config = Reentry::device_test_config(fixture.path().to_owned(), "https://localhost:1")?;
    let (observed, worker) = std::sync::mpsc::channel();
    std::sync::Arc::get_mut(&mut config.device_sessions)
        .ok_or("unique device")?
        .observe_process_forget(observed)?;
    let entry = owner.entry.as_mut().ok_or("entry")?;
    assert!(entry.configure(config));
    entry.arm_cancel_fixture();
    owner.cancel_login();
    assert_ne!(
        worker.recv_timeout(std::time::Duration::from_secs(2))?,
        std::thread::current().id()
    );
    owner.close();
    Ok(())
}

#[cfg(test)]
fn check_pending_admission(sign_out: bool) -> crate::account_services::test_server::TestResult {
    use crate::{account_services::test_server::test_thread, native_entry::test_support::Stack};
    use std::{
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    test_thread(move || {
        let stack = Stack::start_two_channels()?;
        let folder = tempfile::tempdir()?;
        let mut config = stack.config();
        config.device_sessions = std::sync::Arc::new(
            crate::device_login::DeviceLogin::test_fixture(folder.path().to_owned()),
        );
        let mut owner = HostOwner::new(None)?;
        let entry = owner.entry.as_mut().ok_or("entry")?;
        assert!(entry.configure(config));
        assert!(entry.request());
        entry.wait_for_ready_admission()?;
        owner.phase = Phase::Entering;
        if sign_out {
            owner.sign_out_device();
        } else {
            owner.cancel_login();
        }
        let intent = owner.forget_intent.take().ok_or("pending intent")?;
        let (release, wait) = std::sync::mpsc::channel();
        owner.forget_intent = Some(PendingIntent {
            worker: std::thread::spawn(move || {
                wait.recv().map_err(|error| error.to_string())?;
                intent.worker.join().map_err(|_| "forget worker panicked")?
            }),
            ..intent
        });
        owner.poll([800, 600]);
        let held = owner.phase == Phase::Entering && owner.play.is_none();
        release.send(())?;
        owner.finish_forget_intent();
        assert!(held, "ready admission escaped pending intent");
        owner.poll([800, 600]);
        assert_eq!(owner.phase, Phase::Offline);
        assert!(owner.play.is_none());
        let deadline = Instant::now() + Duration::from_secs(3);
        while stack.node.disconnects.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < deadline, "connection stayed open");
            std::thread::yield_now();
        }
        owner.close();
        stack.finish();
        Ok(())
    })
}

#[cfg(test)]
#[test]
fn regression_pending_cancel_holds_admission() -> crate::account_services::test_server::TestResult {
    check_pending_admission(false)
}

#[cfg(test)]
#[test]
fn regression_pending_sign_out_holds_admission() -> crate::account_services::test_server::TestResult
{
    check_pending_admission(true)
}

#[cfg(test)]
#[test]
fn picker_and_shutdown_finish_with_session_lock_held() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("preferences.json");
    let mut config =
        Reentry::device_test_config(fixture.path().join("device"), "https://localhost:1")?;
    config.preference_path = Some(path.clone());
    let mut owner = HostOwner::new(Some(path.clone()))?;
    assert!(owner.entry.as_mut().ok_or("entry")?.configure(config));
    let held = crate::atomic_file::lock(&path.with_extension("session-options.json"))?;
    let (release, wait) = std::sync::mpsc::channel::<()>();
    let unlock = std::thread::spawn(move || {
        let _ = wait.recv_timeout(Duration::from_secs(5));
        drop(held);
    });
    let start = Instant::now();
    owner.cancel_picker(1);
    owner.finish_forget_intent();
    owner.close();
    drop(owner);
    assert!(start.elapsed() < Duration::from_secs(3));
    drop(release);
    unlock.join().map_err(|_| "lock holder panicked")?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let sidecar = path.with_extension("session-options.json");
    while !crate::play::MiscellaneousPreferences::read(&sidecar)
        .is_ok_and(|options| !options.remember_device)
    {
        assert!(Instant::now() < deadline, "opt-out retry did not persist");
        std::thread::yield_now();
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn cancel_reserves_picker_request_until_worker_finishes() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::character_selection::{CharacterRow, SelectionAction};
    use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
    let folder = tempfile::tempdir()?;
    let mut owner = HostOwner::new(None)?;
    let config = Reentry::device_test_config(folder.path().into(), "https://localhost:1")?;
    assert!(owner.entry.as_mut().ok_or("entry")?.configure(config));
    let callback = owner.selector.clone();
    let selector = std::thread::spawn(move || {
        let row = CharacterRow {
            character: OwnerCharacter {
                character_id: "character".into(),
                world_id: "world".into(),
                name: "Knight".into(),
                availability: CharacterAvailability::Available,
            },
            world_name: "World".into(),
            available: true,
            channels: vec![],
        };
        callback(
            &[row],
            "character",
            oteryn_foundation::CancellationToken::new(),
        )
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !owner.selection.poll() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let id = owner.selection.pending().ok_or("request")?.id;
    owner.cancel_picker(id);
    for action in [
        SelectionAction::Play {
            character_id: "character".into(),
            channel_id: None,
        },
        SelectionAction::Create,
        SelectionAction::AccountServices,
    ] {
        assert!(!owner.selection.respond(id, action));
    }
    owner.finish_forget_intent();
    assert_eq!(
        selector.join().map_err(|_| "selector")?,
        SelectionAction::Cancel
    );
    assert!(owner.play.is_none());
    Ok(())
}
