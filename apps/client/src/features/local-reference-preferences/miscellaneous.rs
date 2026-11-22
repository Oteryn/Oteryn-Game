//! Local confirmations and session choices; Platform retains all session authority.
use serde::{Deserialize, Serialize};
use std::{io, path::Path};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MiscellaneousPreferences {
    pub stay_logged_in: bool,
    pub remember_device: bool,
    pub quick_login: bool,
    pub confirm_logout: bool,
    pub confirm_exit: bool,
}
impl Default for MiscellaneousPreferences {
    fn default() -> Self {
        Self {
            stay_logged_in: false,
            remember_device: false,
            quick_login: false,
            confirm_logout: true,
            confirm_exit: true,
        }
    }
}
impl MiscellaneousPreferences {
    pub fn path() -> Option<std::path::PathBuf> {
        super::PreferenceStore::path().map(|p| p.with_extension("session-options.json"))
    }
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| Self::read(&p).ok())
            .unwrap_or_default()
    }
    pub fn read(path: &std::path::Path) -> std::io::Result<Self> {
        if path.with_added_extension("opt-out").try_exists()? {
            match crate::atomic_file::try_lock(path) {
                Ok(_lock) => Self::replay_opt_out(path)?,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    let mut options = Self::read_sidecar(path)?;
                    options.remember_device = false;
                    return Ok(options);
                }
                Err(error) => return Err(error),
            }
        }
        Self::read_sidecar(path)
    }

    fn read_sidecar(path: &Path) -> io::Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(4097)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid session options size",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let _lock = crate::atomic_file::lock(path)?;
        Self::replay_opt_out(path)?;
        self.publish(path)
    }

    pub(super) fn publish_edits(
        &self,
        before: Self,
        path: &std::path::Path,
    ) -> std::io::Result<()> {
        Self::replay_opt_out(path)?;
        let mut active = Self::read_for_edit(path)?;
        let Self {
            stay_logged_in,
            remember_device,
            quick_login,
            confirm_logout,
            confirm_exit,
        } = *self;
        if before.stay_logged_in != stay_logged_in {
            active.stay_logged_in = stay_logged_in;
        }
        if before.remember_device != remember_device {
            active.remember_device = remember_device;
        }
        if before.quick_login != quick_login {
            active.quick_login = quick_login;
        }
        if before.confirm_logout != confirm_logout {
            active.confirm_logout = confirm_logout;
        }
        if before.confirm_exit != confirm_exit {
            active.confirm_exit = confirm_exit;
        }
        active.publish(path)
    }

    #[cfg(any(bevy_shell, test))]
    pub(crate) fn disable_remember_device_at(path: &std::path::Path) -> std::io::Result<()> {
        Self::record_opt_out(path)?;
        Self::try_replay_opt_out(path)
    }

    #[cfg(any(bevy_shell, test))]
    pub(super) fn record_opt_out(path: &Path) -> io::Result<()> {
        crate::atomic_file::publish_durable(&path.with_added_extension("opt-out"), b"false")
    }

    #[cfg(any(bevy_shell, test))]
    pub(super) fn try_replay_opt_out(path: &Path) -> io::Result<()> {
        match crate::atomic_file::try_lock(path) {
            Ok(_lock) => Self::replay_opt_out(path),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(error),
        }
    }

    // The caller holds the publication lock. Keep the intent until the merged
    // sidecar is durable, so an interrupted replay can safely run again.
    fn replay_opt_out(path: &Path) -> io::Result<()> {
        let pending = path.with_added_extension("opt-out");
        if pending.try_exists()? {
            let mut options = Self::read_for_edit(path)?;
            options.remember_device = false;
            options.publish(path)?;
            std::fs::remove_file(pending)?;
        }
        Ok(())
    }

    fn read_for_edit(path: &Path) -> io::Result<Self> {
        match Self::read_sidecar(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                crate::atomic_file::preserve_malformed(path)?;
                Ok(Self::default())
            }
            result => result,
        }
    }

    fn publish(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::atomic_file::publish_durable(path, &serde_json::to_vec(self)?)
    }
}
impl super::SettingsState {
    pub fn request_native_sign_out(&mut self) {
        self.native_sign_out = true;
    }
    pub fn take_native_sign_out(&mut self) -> bool {
        std::mem::take(&mut self.native_sign_out)
    }
    /// Queues a Platform web page (support tickets, cookie policy) for the system browser.
    pub fn request_web_link(&mut self, link: crate::web_links::WebLink) {
        self.web_link = Some(link);
    }
    pub fn take_web_link(&mut self) -> Option<crate::web_links::WebLink> {
        self.web_link.take()
    }
}
