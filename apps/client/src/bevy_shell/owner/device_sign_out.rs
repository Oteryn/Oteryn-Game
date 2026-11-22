use super::*;

impl HostOwner {
    pub fn cancel_login(&mut self) {
        if self.entry.as_ref().is_some_and(Reentry::can_cancel_entry)
            && !self.prepare_device_forget(false)
        {
            return;
        }
        self.login.cancel_save();
        self.selection.cancel();
        self.services.cancel();
        if let Some(entry) = &mut self.entry {
            entry.cancel_entry();
        }
    }

    pub(in crate::bevy_shell) fn prepare_picker_cancel(&mut self) -> bool {
        self.prepare_device_forget(false)
    }

    fn prepare_device_forget(&mut self, cancel_entry: bool) -> bool {
        if let Some(entry) = &mut self.entry {
            let result = if cancel_entry {
                entry.native_sign_out()
            } else {
                entry.forget_native_session()
            };
            match result {
                Ok(revocation) => self.device_sign_out = revocation,
                Err(error) => {
                    let message = format!("Could not record device sign-out: {error}");
                    self.settings.feedback.clone_from(&message);
                    self.login.form.feedback(message);
                    return false;
                }
            }
        }
        self.settings.disable_remember_device();
        true
    }

    /// The login window turns remember-device off and forgets the device for its Platform.
    pub fn sign_out_device(&mut self) {
        if self.play.is_some() {
            return;
        }
        self.login.cancel_save();
        // Before the first sign-in of this process the entry has no Platform yet.
        let configured = self.login.form.settings.config().is_ok_and(|config| {
            self.entry
                .as_mut()
                .is_some_and(|entry| entry.configure(config))
        });
        if !self.prepare_device_forget(true) {
            return;
        }
        self.selection.cancel();
        self.services.cancel();
        self.login.form.feedback(if configured && self.device_sign_out.is_some() {
            "Signing out this device…".into()
        } else {
            "Remember-device is off. Correct the connection settings to also forget this device \
             on the Platform."
                .into()
        });
    }
    /// Turning stay-logged-in or remember-device off in the options forgets this device on the
    /// Platform and, as the Win32 page did, logs the character in play out.
    pub(super) fn forget_device(&mut self) -> bool {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            entry.configure(config);
        }
        if !self.prepare_device_forget(true) {
            return false;
        }
        if let Some(play) = &mut self.play {
            play.view.logout();
        }
        true
    }
    pub(super) fn poll_device_sign_out(&mut self) {
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
    assert!(
        owner.entry.as_mut().ok_or("entry")?.cancel_entry(),
        "the unsuccessful forget must leave the attempt active"
    );
    assert!(owner.settings.preferences.miscellaneous.remember_device);
    Ok(())
}
