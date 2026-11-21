use super::*;

impl HostOwner {
    /// The login window turns remember-device off, cancels entry and forgets the device.
    /// remember-device off, cancels any entry and forgets the remembered device on the
    /// Platform the connection settings name. A normal game logout keeps the device.
    pub fn sign_out_device(&mut self) {
        if self.play.is_some() {
            return;
        }
        let mut preferences = self.settings.preferences.clone();
        preferences.miscellaneous.remember_device = false;
        self.settings.apply(preferences);
        self.selection.cancel();
        self.services.cancel();
        // Before the first sign-in of this process the entry has no Platform yet.
        let configured = self.login.form.settings.config().is_ok_and(|config| {
            self.entry
                .as_mut()
                .is_some_and(|entry| entry.configure(config))
        });
        self.device_sign_out = self.entry.as_mut().and_then(Reentry::native_sign_out);
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
    pub(super) fn forget_device(&mut self) {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            entry.configure(config);
        }
        if let Some(entry) = &mut self.entry {
            entry.native_sign_out();
        }
        if let Some(play) = &mut self.play {
            play.view.logout();
        }
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
